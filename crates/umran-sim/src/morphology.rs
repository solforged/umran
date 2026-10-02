//! How a language builds words from other words: affixes for
//! concatenative languages, vowel patterns over consonant skeletons for
//! root-and-pattern ones. Each language mints its own at founding.

use crate::concepts::{Class, Relation};
use crate::form::{Form, Seg};
use crate::phoneme::{CATALOG, Manner, PhonemeId, Place, Segment};
use crate::phonotactics::Phonotactics;
use crate::profile::{MorphologyKind, MorphologyPrior};
use crate::rng::weighted_index;
use rand::Rng;

/// Draws an affix or pattern gets to differ from the others.
const DISTINCT_TRIES: usize = 16;

#[derive(Clone, Debug, PartialEq)]
pub struct Affix {
    pub form: Form,
    pub suffix: bool,
}

/// One slot of a root-and-pattern template.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    /// The nth root consonant.
    Root(usize),
    /// A fixed segment: a pattern vowel, or a prefix such as Arabic ma-.
    Fixed(PhonemeId),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pattern(pub Vec<Slot>);

#[derive(Clone, Debug, PartialEq)]
pub struct Morphology {
    pub kind: MorphologyKind,
    pub affixes: Vec<(Relation, Affix)>,
    pub patterns: Vec<(Relation, Pattern)>,
    /// Root-and-pattern only: the plain word shape for each word class.
    pub basic: Vec<(Class, Pattern)>,
    /// Inserted between consonants that may not meet within a word.
    pub link: Option<PhonemeId>,
    /// Inserted between vowels meeting at a boundary, as many languages
    /// put j, w, h, or a glottal stop there.
    pub glide: Option<PhonemeId>,
    /// How this language puts words together into names.
    pub names: NameRules,
    /// The affix that renews a worn or ambiguous word, often a diminutive
    /// at first: Latin auris "ear" > auricula > French oreille, Mandarin
    /// -zi in zhuōzi "table". Drawn after the name rules.
    pub renewing: Affix,
    open_medial: bool,
}

/// How a language builds compounds and names. Drawn after everything else
/// a language founds with, so adding it changed no earlier draw.
#[derive(Clone, Debug, PartialEq)]
pub struct NameRules {
    /// Whether a compound puts its head first (Welsh Aberystwyth "mouth of
    /// the Ystwyth", Semitic Beth-lehem "house of bread") or last (English
    /// Hill-folk). Prefixing languages tend to put heads first.
    pub head_first: bool,
    /// "Of, belonging to": the affix that makes a people's name into its
    /// language's (English -ish, Arabic -ī, Swahili ki-).
    pub belonging: Affix,
    /// Whether the language's name is the people's name with `belonging`
    /// (English, Swahili) or a compound with a word for speech (Diné
    /// bizaad, "the people's language"); the concept used if a compound.
    pub speech: Option<&'static str>,
}

/// Share of languages that name their speech with a compound rather than
/// an affix.
const SPEECH_COMPOUND: f32 = 0.4;

impl Morphology {
    pub fn found(prior: &MorphologyPrior, tactics: &Phonotactics, rng: &mut impl Rng) -> Self {
        let onsets: Vec<(PhonemeId, f32)> = tactics
            .onsets
            .iter()
            .filter(|(ids, _)| ids.len() == 1)
            .map(|(ids, w)| (ids[0], *w))
            .collect();
        let vowels = &tactics.nuclei;
        let pick = |rng: &mut dyn rand::RngCore, list: &[(PhonemeId, f32)]| {
            list[weighted_index(&mut &mut *rng, list.iter().map(|(_, w)| *w))].0
        };
        let glide = ["j", "w", "h", "ʔ"]
            .iter()
            .filter_map(|ipa| CATALOG.id_by_ipa(ipa))
            .find(|id| onsets.iter().any(|(o, _)| o == id));
        let link = vowels
            .iter()
            .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.0.cmp(&a.0.0)))
            .map(|(id, _)| *id);

        let mut affixes: Vec<(Relation, Affix)> = Vec::new();
        let mut patterns: Vec<(Relation, Pattern)> = Vec::new();
        let mut basic: Vec<(Class, Pattern)> = Vec::new();
        match prior.kind {
            MorphologyKind::Concatenative => {
                for relation in Relation::ALL {
                    let suffix = rng.r#gen::<f32>() < prior.suffixing;
                    let mut form = Form::default();
                    for _ in 0..DISTINCT_TRIES {
                        let v = pick(rng, vowels);
                        let c = pick(rng, &onsets);
                        let phones = match (suffix, rng.gen_range(0..3)) {
                            (true, 0) => vec![v],
                            (true, 1) => vec![v, c],
                            (true, _) => vec![c, v],
                            (false, 0) => vec![c, v],
                            (false, 1) => vec![v, c],
                            (false, _) => vec![c, v],
                        };
                        form = Form::from_phones(phones);
                        if !affixes.iter().any(|(_, a)| a.form == form) {
                            break;
                        }
                    }
                    affixes.push((relation, Affix { form, suffix }));
                }
            }
            MorphologyKind::RootPattern => {
                // A prefix consonant like Arabic ma-, preferring m.
                let prefix = onsets
                    .iter()
                    .find(|(id, _)| {
                        matches!(CATALOG.get(*id), Segment::Consonant(c)
                            if c.manner == Manner::Nasal && c.place == Place::Bilabial)
                    })
                    .map(|(id, _)| *id)
                    .unwrap_or_else(|| pick(rng, &onsets));
                let mut taken: Vec<Pattern> = Vec::new();
                let mut shape =
                    |rng: &mut dyn rand::RngCore, template: &[Option<usize>], prefixed: bool| {
                        let mut pattern = Pattern(Vec::new());
                        for _ in 0..DISTINCT_TRIES {
                            let mut slots = Vec::new();
                            if prefixed {
                                slots.push(Slot::Fixed(prefix));
                                slots.push(Slot::Fixed(pick(rng, vowels)));
                            }
                            for slot in template {
                                slots.push(match slot {
                                    Some(i) => Slot::Root(*i),
                                    None => Slot::Fixed(pick(rng, vowels)),
                                });
                            }
                            pattern = Pattern(slots);
                            if !taken.contains(&pattern) {
                                break;
                            }
                        }
                        taken.push(pattern.clone());
                        pattern
                    };
                // C V C V C, the plain shape; others vary vowels, add a
                // final vowel, or prefix.
                let cvcvc = [Some(0), None, Some(1), None, Some(2)];
                let cvcvcv = [Some(0), None, Some(1), None, Some(2), None];
                let ccvc = [Some(0), Some(1), None, Some(2)];
                for class in [
                    Class::Entity,
                    Class::Event,
                    Class::Property,
                    Class::Function,
                ] {
                    basic.push((class, shape(rng, &cvcvc, false)));
                }
                for relation in Relation::ALL {
                    let pattern = match relation {
                        Relation::Place | Relation::Instrument => shape(rng, &ccvc, true),
                        Relation::Collective | Relation::Action => shape(rng, &cvcvcv, false),
                        _ => shape(rng, &cvcvc, false),
                    };
                    patterns.push((relation, pattern));
                }
            }
        }
        let names = NameRules::draw(prior, &onsets, vowels, &affixes, rng);
        let renewing = renewing_affix(prior, &onsets, vowels, &affixes, &names, rng);
        Self {
            kind: prior.kind,
            affixes,
            patterns,
            basic,
            link,
            glide,
            names,
            renewing,
            open_medial: tactics.open_medial,
        }
    }

    /// A compound of `modifier` and `head`, in this language's order.
    pub fn compound(&self, modifier: &Form, head: &Form) -> Form {
        if self.names.head_first {
            self.join(head, modifier)
        } else {
            self.join(modifier, head)
        }
    }

    /// "Of `base`": the belonging affix attached.
    pub fn belonging(&self, base: &Form) -> Form {
        let affix = &self.names.belonging;
        if affix.suffix {
            self.join(base, &affix.form)
        } else {
            self.join(&affix.form, base)
        }
    }

    /// The consonant skeleton of a form: its first three consonants, with
    /// the last repeated if it has fewer (as Semitic doubled roots do).
    pub fn skeleton(form: &Form) -> Option<[PhonemeId; 3]> {
        let consonants: Vec<PhonemeId> = form
            .phones()
            .filter(|p| !CATALOG.get(*p).is_vowel())
            .collect();
        match consonants.as_slice() {
            [] => None,
            [a] => Some([*a, *a, *a]),
            [a, b] => Some([*a, *b, *b]),
            [a, b, c, ..] => Some([*a, *b, *c]),
        }
    }

    pub fn realize(pattern: &Pattern, skeleton: [PhonemeId; 3]) -> Form {
        Form::from_phones(pattern.0.iter().map(|slot| match slot {
            Slot::Root(i) => skeleton[*i],
            Slot::Fixed(p) => *p,
        }))
    }

    /// A root-and-pattern word of `class` from a skeleton.
    pub fn word(&self, class: Class, skeleton: [PhonemeId; 3]) -> Option<Form> {
        let pattern = &self.basic.iter().find(|(c, _)| *c == class)?.1;
        Some(Self::realize(pattern, skeleton))
    }

    /// The word for `relation` built from `base`, or `None` if this language
    /// cannot build one (no affix, or a base without consonants). In a
    /// root-and-pattern language, pass the base's `root` skeleton when it is
    /// known: a prefixed pattern like ma-ktab hides it from a naive reading.
    pub fn derive(
        &self,
        base: &Form,
        root: Option<[PhonemeId; 3]>,
        relation: Relation,
    ) -> Option<Form> {
        match self.kind {
            MorphologyKind::RootPattern => {
                let pattern = &self.patterns.iter().find(|(r, _)| *r == relation)?.1;
                Some(Self::realize(
                    pattern,
                    root.or_else(|| Self::skeleton(base))?,
                ))
            }
            MorphologyKind::Concatenative => {
                let affix = &self.affixes.iter().find(|(r, _)| *r == relation)?.1;
                Some(if affix.suffix {
                    self.join(base, &affix.form)
                } else {
                    self.join(&affix.form, base)
                })
            }
        }
    }

    /// `base` renewed with the renewing affix.
    pub fn renew(&self, base: &Form) -> Form {
        if self.renewing.suffix {
            self.join(base, &self.renewing.form)
        } else {
            self.join(&self.renewing.form, base)
        }
    }

    /// Joins two pieces, recording the boundary. Two vowels meeting lose
    /// the second, or, when it is all the affix has, get a glide between
    /// them; two consonants meeting get the link vowel in a language that
    /// keeps syllables open inside words.
    pub(crate) fn join(&self, left: &Form, right: &Form) -> Form {
        let vowel = |f: &Form, i: usize| CATALOG.get(f.segs[i].phone).is_vowel();
        let mut segs: Vec<Seg> = left.segs.clone();
        let mut tail: Vec<Seg> = right.segs.clone();
        let (Some(last), Some(_)) = (segs.len().checked_sub(1), tail.first()) else {
            return Form {
                segs: [segs, tail].concat(),
                boundaries: Vec::new(),
                stress: left.stress.or(right.stress),
            };
        };
        let (left_vowel, right_vowel) = (vowel(left, last), vowel(right, 0));
        if left_vowel && right_vowel && tail.len() > 1 {
            tail.remove(0);
        } else if left_vowel && right_vowel {
            if let Some(glide) = self.glide {
                segs.push(Seg {
                    phone: glide,
                    long: false,
                });
            }
        } else if !left_vowel
            && !right_vowel
            && self.open_medial
            && let Some(link) = self.link
        {
            segs.push(Seg {
                phone: link,
                long: false,
            });
        }
        // The leftmost lexical accent wins; otherwise carry the right
        // accent across the link vowel and any hiatus deletion.
        let stress = left.stress.or_else(|| {
            right.stress.map(|s| {
                let prefix = segs
                    .iter()
                    .filter(|s| CATALOG.get(s.phone).is_vowel())
                    .count();
                let deleted = usize::from(left_vowel && right_vowel && right.segs.len() > 1);
                (prefix + s).saturating_sub(deleted)
            })
        });
        let boundary = segs.len();
        segs.extend(tail);
        Form {
            segs,
            boundaries: vec![boundary],
            stress,
        }
    }
}

/// A consonant-vowel renewing affix, unlike the language's others. It
/// adds a whole syllable, as renewing affixes do.
fn renewing_affix(
    prior: &MorphologyPrior,
    onsets: &[(PhonemeId, f32)],
    vowels: &[(PhonemeId, f32)],
    affixes: &[(Relation, Affix)],
    names: &NameRules,
    rng: &mut impl Rng,
) -> Affix {
    let pick = |rng: &mut dyn rand::RngCore, list: &[(PhonemeId, f32)]| {
        list[weighted_index(&mut &mut *rng, list.iter().map(|(_, w)| *w))].0
    };
    let suffixing = match prior.kind {
        MorphologyKind::RootPattern => 1.0,
        MorphologyKind::Concatenative => prior.suffixing,
    };
    let suffix = rng.r#gen::<f32>() < suffixing;
    let mut form = Form::default();
    for _ in 0..DISTINCT_TRIES {
        let c = pick(rng, onsets);
        let v = pick(rng, vowels);
        form = Form::from_phones([c, v]);
        let taken = affixes.iter().any(|(_, a)| a.form == form) || names.belonging.form == form;
        if !taken {
            break;
        }
    }
    Affix { form, suffix }
}

impl NameRules {
    fn draw(
        prior: &MorphologyPrior,
        onsets: &[(PhonemeId, f32)],
        vowels: &[(PhonemeId, f32)],
        affixes: &[(Relation, Affix)],
        rng: &mut impl Rng,
    ) -> Self {
        let pick = |rng: &mut dyn rand::RngCore, list: &[(PhonemeId, f32)]| {
            list[weighted_index(&mut &mut *rng, list.iter().map(|(_, w)| *w))].0
        };
        // Root-and-pattern languages attach this one as a suffix, like the
        // Arabic nisba -ī, and put heads first, like the construct state.
        let (suffixing, head_first) = match prior.kind {
            MorphologyKind::RootPattern => (1.0, 0.9),
            MorphologyKind::Concatenative => {
                (prior.suffixing, 0.15 + 0.7 * (1.0 - prior.suffixing))
            }
        };
        let head_first = rng.r#gen::<f32>() < head_first;
        let suffix = rng.r#gen::<f32>() < suffixing;
        let mut form = Form::default();
        for _ in 0..DISTINCT_TRIES {
            let v = pick(rng, vowels);
            let c = pick(rng, onsets);
            form = Form::from_phones(match (suffix, rng.gen_range(0..3)) {
                (true, 0) => vec![v],
                (_, 1) => vec![v, c],
                _ => vec![c, v],
            });
            if !affixes.iter().any(|(_, a)| a.form == form) {
                break;
            }
        }
        let speech = (rng.r#gen::<f32>() < SPEECH_COMPOUND).then(|| {
            if rng.r#gen::<f32>() < 0.6 {
                "tongue"
            } else {
                "word"
            }
        });
        Self {
            head_first,
            belonging: Affix { form, suffix },
            speech,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::Inventory;
    use crate::profile::SoundProfile;
    use crate::rng::stream;

    fn morphology(
        profile: &SoundProfile,
        kind: MorphologyKind,
        seed: u64,
    ) -> (Morphology, Phonotactics) {
        let inventory = Inventory::sample(&profile.inventory, &mut stream(seed, &[]));
        let tactics = Phonotactics::compile(&profile.phonotactics, &inventory);
        let prior = MorphologyPrior {
            kind,
            ..profile.morphology
        };
        (
            Morphology::found(&prior, &tactics, &mut stream(seed, &[1])),
            tactics,
        )
    }

    #[test]
    fn affixes_are_distinct_and_attach_at_a_boundary() {
        let profile = SoundProfile::base();
        for seed in 0..50 {
            let (m, _) = morphology(&profile, MorphologyKind::Concatenative, seed);
            let forms: Vec<&Form> = m.affixes.iter().map(|(_, a)| &a.form).collect();
            for (i, f) in forms.iter().enumerate() {
                assert!(!forms[i + 1..].contains(f), "seed {seed}: duplicate affix");
            }
            let base = Form::from_ipa("kat").unwrap();
            let derived = m.derive(&base, None, Relation::Agent).unwrap();
            assert_eq!(derived.boundaries.len(), 1);
            assert!(
                derived.ipa().contains("kat"),
                "{} keeps its base",
                derived.ipa()
            );
        }
    }

    #[test]
    fn root_patterns_share_a_skeleton() {
        let profile = SoundProfile::base();
        let (m, _) = morphology(&profile, MorphologyKind::RootPattern, 3);
        let base = Form::from_ipa("katab").unwrap();
        let place = m.derive(&base, None, Relation::Place).unwrap();
        let agent = m.derive(&base, None, Relation::Agent).unwrap();
        // Deriving again from the prefixed word keeps the true root.
        let root = Morphology::skeleton(&base);
        let again = m.derive(&place, root, Relation::Agent).unwrap();
        assert_eq!(again, agent);
        assert_eq!(Morphology::skeleton(&agent), Morphology::skeleton(&base));
        assert_ne!(place, agent);
        assert!(
            place.ipa().contains("kt"),
            "a prefixed pattern clusters the first two: {}",
            place.ipa()
        );
    }

    #[test]
    fn junctions_follow_the_language() {
        let mut m = morphology(&SoundProfile::base(), MorphologyKind::Concatenative, 1).0;
        m.link = CATALOG.id_by_ipa("a");
        m.open_medial = true;
        let f = |s: &str| Form::from_ipa(s).unwrap();
        assert_eq!(m.join(&f("kat"), &f("ma")).ipa(), "katama");
        assert_eq!(m.join(&f("ka"), &f("ir")).ipa(), "kar");
        m.glide = CATALOG.id_by_ipa("j");
        assert_eq!(m.join(&f("dʒemo"), &f("o")).ipa(), "dʒemojo");
        m.open_medial = false;
        assert_eq!(m.join(&f("kat"), &f("ma")).ipa(), "katma");
    }
}

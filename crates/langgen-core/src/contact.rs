use crate::family::{Branch, Family};
use crate::generate::{Generator, Word};
use crate::inventory::Inventory;
use crate::phoneme::{CATALOG, PhonemeId, Segment};

#[derive(Clone, Debug)]
pub enum Transfer {
    Lexicon { n: usize },
    Pidgin { lexicon_cap: usize },
}

#[derive(Clone, Debug)]
pub struct ContactEvent {
    pub donor_branch: String,
    pub recipient_branch: String,
    pub intensity: f32,
    pub transfer: Transfer,
}

pub fn apply_contact(family: &mut Family, event: &ContactEvent) {
    let Some((donor, recipient)) = donor_and_recipient(
        &mut family.branches,
        &event.donor_branch,
        &event.recipient_branch,
    ) else {
        return;
    };
    match event.transfer {
        Transfer::Lexicon { n } => apply_lexicon(donor, recipient, n, event.intensity),
        Transfer::Pidgin { lexicon_cap } => {
            apply_pidgin(donor, recipient, lexicon_cap, event.intensity)
        }
    }
}

fn donor_and_recipient<'a>(
    branches: &'a mut [Branch],
    donor_id: &str,
    recipient_id: &str,
) -> Option<(&'a Branch, &'a mut Branch)> {
    let donor_idx = branches.iter().position(|b| b.id == donor_id)?;
    let recipient_idx = branches.iter().position(|b| b.id == recipient_id)?;
    if donor_idx == recipient_idx {
        return None;
    }
    if donor_idx < recipient_idx {
        let (left, right) = branches.split_at_mut(recipient_idx);
        Some((&left[donor_idx], &mut right[0]))
    } else {
        let (left, right) = branches.split_at_mut(donor_idx);
        Some((&right[0], &mut left[recipient_idx]))
    }
}

fn scaled(n: usize, intensity: f32) -> usize {
    if intensity <= 0.0 {
        0
    } else {
        ((n as f32) * intensity).ceil() as usize
    }
}

fn apply_lexicon(donor: &Branch, recipient: &mut Branch, n: usize, intensity: f32) {
    let n_eff = scaled(n, intensity);
    let have = recipient
        .cognates
        .iter()
        .filter(|(id, _)| id.starts_with("borrow:"))
        .count();
    let need = n_eff.saturating_sub(have);
    if need == 0 {
        return;
    }
    let mut candidates: Vec<(String, Word)> = donor
        .cognates
        .iter()
        .filter(|(id, _)| !id.starts_with("borrow:"))
        .filter(|(id, _)| {
            let borrow_id = format!("borrow:{id}");
            !recipient.cognates.iter().any(|(rid, _)| rid == &borrow_id)
        })
        .cloned()
        .collect();
    candidates.sort_by(|a, b| a.0.cmp(&b.0));
    let inventory = &recipient.language.inventory;
    for (id, word) in candidates.into_iter().take(need) {
        let nativized = nativize_word(&word, inventory);
        recipient.cognates.push((format!("borrow:{id}"), nativized));
    }
}

fn apply_pidgin(donor: &Branch, recipient: &mut Branch, lexicon_cap: usize, intensity: f32) {
    let cap_eff = scaled(lexicon_cap, intensity);
    let mixed = Inventory::from_ids(
        donor
            .language
            .inventory
            .consonants
            .iter()
            .copied()
            .chain(donor.language.inventory.vowels.iter().copied())
            .chain(recipient.language.inventory.consonants.iter().copied())
            .chain(recipient.language.inventory.vowels.iter().copied()),
    );
    recipient.language.generator = Generator::compile(&recipient.language.aesthetic, &mixed);
    recipient.language.inventory = mixed;

    let mut donor_cogs: Vec<(String, Word)> = donor
        .cognates
        .iter()
        .filter(|(id, _)| !id.starts_with("borrow:"))
        .cloned()
        .collect();
    let mut recip_cogs: Vec<(String, Word)> = recipient
        .cognates
        .iter()
        .filter(|(id, _)| !id.starts_with("borrow:"))
        .cloned()
        .collect();
    donor_cogs.sort_by(|a, b| a.0.cmp(&b.0));
    recip_cogs.sort_by(|a, b| a.0.cmp(&b.0));

    let inventory = &recipient.language.inventory;
    let mut seen: Vec<String> = Vec::new();
    let mut out: Vec<(String, Word)> = Vec::new();
    let rounds = donor_cogs.len().max(recip_cogs.len());
    for i in 0..rounds {
        if out.len() >= cap_eff {
            break;
        }
        if i < donor_cogs.len() {
            push_unique(&mut out, &mut seen, &donor_cogs[i], inventory, cap_eff);
        }
        if out.len() >= cap_eff {
            break;
        }
        if i < recip_cogs.len() {
            push_unique(&mut out, &mut seen, &recip_cogs[i], inventory, cap_eff);
        }
    }
    recipient.cognates = out;
}

fn push_unique(
    out: &mut Vec<(String, Word)>,
    seen: &mut Vec<String>,
    pair: &(String, Word),
    inventory: &Inventory,
    cap_eff: usize,
) {
    if out.len() >= cap_eff || seen.iter().any(|id| id == &pair.0) {
        return;
    }
    seen.push(pair.0.clone());
    out.push((pair.0.clone(), nativize_word(&pair.1, inventory)));
}

pub(crate) fn nativize_word(word: &Word, inventory: &Inventory) -> Word {
    let mut out = word.clone();
    for syl in &mut out.syllables {
        nativize_ids(&mut syl.onset, inventory);
        nativize_ids(&mut syl.nucleus, inventory);
        nativize_ids(&mut syl.coda, inventory);
    }
    out
}

fn nativize_ids(ids: &mut [PhonemeId], inventory: &Inventory) {
    for id in ids {
        *id = nativize_phone(*id, inventory);
    }
}

fn nativize_phone(id: PhonemeId, inventory: &Inventory) -> PhonemeId {
    if inventory.contains(id) {
        return id;
    }
    let want_vowel = CATALOG.get(id).is_vowel();
    let candidates = if want_vowel {
        &inventory.vowels
    } else {
        &inventory.consonants
    };
    candidates
        .iter()
        .copied()
        .min_by_key(|&cand| (feature_distance(id, cand), cand.0))
        .unwrap_or(id)
}

fn feature_distance(a: PhonemeId, b: PhonemeId) -> u32 {
    match (CATALOG.get(a), CATALOG.get(b)) {
        (Segment::Consonant(a), Segment::Consonant(b)) => {
            disc_abs(a.place as i32, b.place as i32)
                + disc_abs(a.manner as i32, b.manner as i32)
                + u32::from(a.voiced != b.voiced)
        }
        (Segment::Vowel(a), Segment::Vowel(b)) => {
            disc_abs(a.height as i32, b.height as i32)
                + disc_abs(a.backness as i32, b.backness as i32)
                + u32::from(a.rounded != b.rounded)
        }
        _ => u32::MAX,
    }
}

fn disc_abs(a: i32, b: i32) -> u32 {
    a.abs_diff(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Aesthetic;
    use crate::family::BranchSpec;

    fn two_branch_elvish() -> Family {
        let aesthetic = Aesthetic::by_id("elvish").unwrap();
        Family::grow(
            42,
            aesthetic.clone(),
            &[
                BranchSpec {
                    id: "a",
                    parent: None,
                    aesthetic: aesthetic.clone(),
                    changes: vec![],
                },
                BranchSpec {
                    id: "b",
                    parent: None,
                    aesthetic,
                    changes: vec![],
                },
            ],
        )
    }

    fn branch_cognate_view(family: &Family, id: &str) -> Vec<(String, Vec<PhonemeId>)> {
        family
            .branches
            .iter()
            .find(|b| b.id == id)
            .unwrap()
            .cognates
            .iter()
            .map(|(cid, w)| (cid.clone(), w.phonemes().collect()))
            .collect()
    }

    #[test]
    fn lexicon_borrow_is_deterministic() {
        let mut family = two_branch_elvish();
        let event = ContactEvent {
            donor_branch: "a".into(),
            recipient_branch: "b".into(),
            intensity: 1.0,
            transfer: Transfer::Lexicon { n: 3 },
        };
        apply_contact(&mut family, &event);
        let first = branch_cognate_view(&family, "b");
        assert!(first.iter().any(|(id, _)| id.starts_with("borrow:")));

        apply_contact(&mut family, &event);
        let second = branch_cognate_view(&family, "b");
        assert_eq!(first, second);
    }
}

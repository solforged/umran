//! Two communities in contact: a prestigious donor and an open recipient.
//!
//! cargo run --release -p langgen-sim --example contact -- \
//!     [seed] [donor profile] [recipient profile] [kind] [generations] [seeds for averages]
//! kind: neighbours | trade | rule | religion | intermarriage

use langgen_sim::wold::{BORROWED_SCORE, spearman};
use langgen_sim::{CATALOG, ContactKind, Event, Field, Origin, Params, SoundProfile, World};
use std::collections::BTreeMap;

fn world(
    seed: u64,
    donor: &SoundProfile,
    recipient: &SoundProfile,
    kind: ContactKind,
    generations: u32,
) -> World {
    let mut world = World::new(seed, Params::static_society());
    let d = world.found("Donor", donor, 0.8, 0.4);
    let r = world.found("Recipient", recipient, 0.3, 0.7);
    world.connect(d, r, 0.8, kind);
    world.run(generations);
    world
}

/// Share of each field's concepts whose dominant recipient word is a loan.
fn loan_shares(world: &World) -> BTreeMap<Field, (f32, f32)> {
    let lexicon = &world.variety_of(1).lexicon;
    let mut out: BTreeMap<Field, (f32, f32)> = BTreeMap::new();
    for slot in &lexicon.slots {
        let entry = out.entry(slot.concept.field).or_default();
        entry.1 += 1.0;
        if slot
            .dominant()
            .is_some_and(|id| matches!(lexicon.get(id).origin, Origin::Borrowed { .. }))
        {
            entry.0 += 1.0;
        }
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |i: usize, default: &str| args.get(i).cloned().unwrap_or_else(|| default.into());
    let seed: u64 = arg(0, "42").parse().expect("seed");
    let donor = SoundProfile::by_id(&arg(1, "illithid")).expect("donor profile");
    let recipient = SoundProfile::by_id(&arg(2, "kuo-toa")).expect("recipient profile");
    let kind = match arg(3, "neighbours").as_str() {
        "neighbours" => ContactKind::Neighbours,
        "trade" => ContactKind::Trade,
        "rule" => ContactKind::Rule,
        "religion" => ContactKind::Religion,
        "intermarriage" => ContactKind::Intermarriage,
        other => panic!("unknown contact kind {other}"),
    };
    let generations: u32 = arg(4, "40").parse().expect("generations");
    let seeds: u64 = arg(5, "100").parse().expect("seeds");

    let w = world(seed, &donor, &recipient, kind, generations);
    let (dv, rv) = (w.variety_of(0), w.variety_of(1));
    println!(
        "{} (prestige 0.8) → {} (prestige 0.3), {kind:?} contact, intensity 0.8 · seed {seed} · {generations} generations",
        donor.name, recipient.name
    );
    let ipa = |ids: Vec<langgen_sim::PhonemeId>| {
        ids.iter()
            .map(|id| CATALOG.get(*id).ipa())
            .collect::<Vec<_>>()
            .join(" ")
    };
    let founding = langgen_sim::Variety::found(0, &recipient);
    let _ = founding;
    let (c, v) = rv.inventory();
    println!("recipient consonants now  {}", ipa(c));
    println!("recipient vowels now      {}", ipa(v));
    let foreign: Vec<&str> = rv
        .inventory()
        .0
        .into_iter()
        .chain(rv.inventory().1)
        .filter(|id| !rv.founding_inventory.contains(*id))
        .map(|id| CATALOG.get(id).ipa())
        .collect();
    println!(
        "sounds new since founding {}",
        if foreign.is_empty() {
            "none".into()
        } else {
            foreign.join(" ")
        }
    );

    println!("\nLoans now dominant in the recipient (donor word then → adapted → today)");
    let lexicon = &rv.lexicon;
    let mut shown = 0;
    for slot in &lexicon.slots {
        let Some(l) = slot.dominant().map(|id| lexicon.get(id)) else {
            continue;
        };
        let Origin::Borrowed { .. } = l.origin else {
            continue;
        };
        let Some(Event::Borrowed { source, .. }) = l.log.iter().map(|e| &e.event).next() else {
            continue;
        };
        let adapted = l
            .log
            .iter()
            .find_map(|e| match &e.event {
                Event::SoundLaw { before, .. } => Some(before),
                _ => None,
            })
            .unwrap_or(&l.form);
        if shown < 24 {
            println!(
                "  {:<10} {:<10} {:>8} → {:<8} → {:<8} gen {}",
                slot.concept.id,
                format!("{:?}", slot.concept.field),
                source.ipa(),
                adapted.ipa(),
                l.form.ipa(),
                l.born
            );
        }
        shown += 1;
    }
    let borrowed_back = dv
        .lexicon
        .slots
        .iter()
        .filter(|s| {
            s.dominant()
                .is_some_and(|id| matches!(dv.lexicon.get(id).origin, Origin::Borrowed { .. }))
        })
        .count();
    println!(
        "  {shown} recipient concepts now use a loan; the donor uses {borrowed_back} loans the other way"
    );

    println!("\nLoan share by field over {seeds} seeds, against WOLD's borrowed score");
    let mut totals: BTreeMap<Field, (f32, f32)> = BTreeMap::new();
    for s in 0..seeds {
        for (field, (loans, n)) in loan_shares(&world(s, &donor, &recipient, kind, generations)) {
            let t = totals.entry(field).or_default();
            t.0 += loans;
            t.1 += n;
        }
    }
    let mut rows: Vec<(Field, f32, f32)> = BORROWED_SCORE
        .iter()
        .filter_map(|&(field, wold)| totals.get(&field).map(|(l, n)| (field, l / n, wold)))
        .collect();
    rows.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (field, sim, wold) in &rows {
        println!(
            "  {:<18} {:>5.1}%   WOLD {:.2}",
            format!("{field:?}"),
            sim * 100.0,
            wold
        );
    }
    let all: f32 =
        totals.values().map(|(l, _)| l).sum::<f32>() / totals.values().map(|(_, n)| n).sum::<f32>();
    let pairs: Vec<(f32, f32)> = rows.iter().map(|(_, s, w)| (*s, *w)).collect();
    println!(
        "  overall {:.1}% · Spearman with WOLD {:.2}",
        all * 100.0,
        spearman(&pairs)
    );
}

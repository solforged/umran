//! cargo run --release -p umran-sim --example harmony -- [seeds] [generations]
use umran_sim::harmony::{Feature, Trigger};
use umran_sim::{Event, Form, Params, SoundProfile, World};

#[path = "../../umran-web/examples/support/sample.rs"]
mod sample;

fn report(world: &World, name: &str) {
    let mut gained = 0;
    let mut lost = 0;
    let mut mergers = 0;
    let mut attrition = 0;
    let mut contact = 0;
    let mut active = 0;
    let mut loans = 0;
    for (id, v) in world.varieties.iter().enumerate() {
        if let Some(h) = &v.harmony {
            active += usize::from(h.active());
            loans += h.disharmonic_loans.len();
        }
        for event in &v.harmony_events {
            if v.parent
                .is_some_and(|p| world.varieties[p.variety].harmony_events.contains(event))
            {
                continue;
            }
            gained += usize::from(event.gained);
            lost += usize::from(!event.gained);
            let reason = match event.trigger {
                Trigger::Assimilation { generation, law } => {
                    format!("after {law} in year {}", generation * 25)
                }
                Trigger::ContrastMerger { law } => {
                    mergers += usize::from(!event.gained);
                    format!("contrast lost after {law}")
                }
                Trigger::LexicalAttrition => {
                    attrition += usize::from(!event.gained);
                    "contrast lost through lexical attrition".into()
                }
                Trigger::Contact { donor, .. } => {
                    contact += usize::from(!event.gained);
                    format!("sustained contact with language {donor}")
                }
            };
            println!(
                "  year {:4}, language {id}: {} {} ({reason})",
                event.generation * 25,
                if event.gained { "gained" } else { "lost" },
                event.feature.id()
            );
        }
        if let Some(word) = v.lexicon.living().find(|l| {
            l.log.iter().any(
                |e| matches!(e.event, Event::SoundLaw { law, .. } if law.starts_with("harmony-")),
            )
        }) {
            let (law, before) = word
                .log
                .iter()
                .find_map(|e| match &e.event {
                    Event::SoundLaw { law, before } if law.starts_with("harmony-") => {
                        Some((law, before))
                    }
                    _ => None,
                })
                .unwrap();
            println!(
                "    {}: /{}/ > /{}/ ({law}; current form may include later laws)",
                word.first_sense.gloss,
                before.ipa(),
                word.form.ipa()
            );
        }
    }
    println!(
        "{name}: {} varieties, {gained} gains, {lost} losses, {active} active ({:.2}%), {loans} recorded loan exceptions; losses by cause: merger {mergers}, lexical attrition {attrition}, contact {contact}",
        world.varieties.len(),
        100.0 * active as f32 / world.varieties.len() as f32
    );
}

fn main() {
    let mut args = std::env::args().skip(1);
    let seeds: u64 = args.next().map_or(4, |s| s.parse().expect("seeds"));
    let generations = args.next().map_or(160, |s| s.parse().expect("generations"));
    println!(
        "Word harmony: a paired stem vowel controls the word; unpaired vowels are transparent."
    );
    for (feature, ipa) in [
        (Feature::Backness, "punitøkɑ"),
        (Feature::Rounding, "pykiteli"),
        (Feature::Atr, "pɪtekoku"),
    ] {
        let mut form = Form::from_ipa(ipa).unwrap();
        feature.apply(&mut form);
        println!("  {}: /{ipa}/ > /{}/", feature.id(), form.ipa());
    }
    println!("ATR = advanced tongue root; represented here by explicit tense/lax vowel pairs.");
    for seed in 0..seeds {
        for profile in ["finnic", "germanic", "semitic", "polynesian"] {
            let mut world = World::solo(
                seed,
                &SoundProfile::by_id(profile).unwrap(),
                Params::default(),
            );
            world.run(generations);
            report(
                &world,
                &format!("{profile}, seed {seed}, {} years", generations * 25),
            );
        }
    }
    let sample = sample::sample();
    report(sample.latest(), "workbench sample, 4,000 years");
}

//! Pronoun case timing across isolated founders and the unchanged workbench recipe.
#[path = "../../umran-web/examples/support/sample.rs"]
mod sample;
use umran_sim::grammar::{Category, MarkerKind, NoticeKind, PronounCase, WordOrder};
use umran_sim::{Params, SoundProfile, Variety, World};

#[derive(Default)]
struct Counts {
    founded: usize,
    particle: usize,
    ahead: usize,
    ahead_now: usize,
    particle_ahead: usize,
    relic: usize,
    relic_now: usize,
    noun_loss: usize,
    lost_relic_now: usize,
    fusion_generations: u32,
    fusions: usize,
}
impl Counts {
    fn record(
        &mut self,
        v: &Variety,
        founded: bool,
        particle: bool,
        ahead: bool,
        relic: bool,
        noun_loss: bool,
    ) {
        self.founded += usize::from(founded);
        self.particle += usize::from(particle);
        self.particle_ahead += usize::from(particle && ahead);
        self.ahead += usize::from(founded && ahead);
        self.ahead_now += usize::from(founded && v.grammar.pronoun_case() == PronounCase::Ahead);
        self.relic += usize::from(founded && relic);
        self.relic_now += usize::from(founded && v.grammar.pronoun_case() == PronounCase::Relic);
        self.noun_loss += usize::from(founded && noun_loss);
        self.lost_relic_now +=
            usize::from(founded && noun_loss && v.grammar.pronoun_case() == PronounCase::Relic);
        if let Some(e) = v
            .grammar
            .events
            .iter()
            .find(|e| matches!(e.event, NoticeKind::PronounFusion { .. }))
        {
            self.fusions += 1;
            self.fusion_generations += e.generation;
        }
    }
    fn print(&self, profile: &str, seeds: u64) {
        println!(
            "{profile:12} founders={seeds} object={} particle={} Ahead-ever={} Ahead-4000={} Relic-ever={} Relic-4000={} particle-Ahead={}/{} ({:.1}%) lost-noun-Relic={}/{} ({:.1}%) mean-first-fusion={:.1} generations (n={})",
            self.founded,
            self.particle,
            self.ahead,
            self.ahead_now,
            self.relic,
            self.relic_now,
            self.particle_ahead,
            self.particle,
            percent(self.particle_ahead, self.particle),
            self.lost_relic_now,
            self.noun_loss,
            percent(self.lost_relic_now, self.noun_loss),
            self.fusion_generations as f32 / self.fusions.max(1) as f32,
            self.fusions
        );
    }
}
fn percent(n: usize, d: usize) -> f32 {
    100.0 * n as f32 / d.max(1) as f32
}
fn sentences(v: &Variety) -> Option<[String; 2]> {
    let render = |subject, object| {
        let words = [
            (subject, None),
            ("see", Some(Category::Past)),
            (object, Some(Category::Object)),
        ];
        let order = match v.grammar.order {
            WordOrder::SOV => [0, 2, 1],
            WordOrder::SVO => [0, 1, 2],
            WordOrder::VSO => [1, 0, 2],
        };
        let mut forms = Vec::new();
        for i in order {
            let (cell, category) = words[i];
            let word = v.lexicon.word_for(umran_sim::concepts::by_id(cell)?)?;
            if let Some(category) = category {
                let r = word
                    .paradigms
                    .iter()
                    .find(|p| p.category == category)?
                    .realizations
                    .iter()
                    .filter(|r| r.retired.is_none() && r.share > 0.0)
                    .max_by(|a, b| a.share.total_cmp(&b.share))?;
                forms.extend(
                    v.grammar
                        .surface(&word.form, r)
                        .iter()
                        .map(|f| f.ipa_stressed(v.stress())),
                );
            } else {
                forms.push(word.form.ipa_stressed(v.stress()));
            }
        }
        Some(forms.join(" "))
    };
    Some([render("1sg", "2sg")?, render("2sg", "1sg")?])
}
fn show(v: &Variety, name: &str) {
    if let Some([first, second]) = sentences(v) {
        println!(
            "  {name}: I saw you. /{first}/; You saw me. /{second}/ ({:?})",
            v.grammar.pronoun_case()
        );
    } else {
        println!("  {name}: required sentence word/form missing");
    }
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let seeds = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(40);
    let defaults = Params::default();
    let params = Params {
        pronoun_case: true,
        pronoun_rate: defaults.pronoun_rate,
        pronoun_fusion_rate: args
            .get(2)
            .and_then(|s| s.parse().ok())
            .unwrap_or(defaults.pronoun_fusion_rate),
        pronoun_case_retention: args
            .get(3)
            .and_then(|s| s.parse().ok())
            .unwrap_or(defaults.pronoun_case_retention),
        ..Params::static_society()
    };
    println!(
        "{seeds} seeds × 4,000 years per profile; fusion={} retention={}; isolated society, pronoun renewal enabled",
        params.pronoun_fusion_rate, params.pronoun_case_retention
    );
    let mut pooled = Counts::default();
    for profile in ["familiar", "semitic", "polynesian"] {
        let mut counts = Counts::default();
        for seed in 0..seeds {
            let mut world =
                World::solo(seed, &SoundProfile::by_id(profile).unwrap(), params.clone());
            let grammar = &world.varieties[0].grammar;
            let founded = grammar.has_object_marking();
            let particle = grammar
                .markers
                .iter()
                .any(|m| m.category == Category::Object && m.kind == MarkerKind::Particle);
            let (mut ahead, mut relic, mut noun_loss) = (false, false, false);
            for _ in 0..160 {
                world.step();
                let grammar = &world.varieties[0].grammar;
                ahead |= grammar.pronoun_case() == PronounCase::Ahead;
                relic |= grammar.pronoun_case() == PronounCase::Relic;
                noun_loss |= !grammar.has_object_marking();
            }
            let v = &world.varieties[0];
            counts.record(v, founded, particle, ahead, relic, noun_loss);
            pooled.record(v, founded, particle, ahead, relic, noun_loss);
            if seed < 3 {
                show(v, &format!("{profile} seed {seed}"));
            }
        }
        counts.print(profile, seeds);
    }
    pooled.print("pooled", seeds * 3);
    let mut history = sample::sample();
    let founded = history.world_at(0);
    let mut counts = Counts::default();
    let mut seen = [(false, false, false); 3];
    for generation in 1..=160 {
        let world = history.world_at(generation);
        for (i, (ahead, relic, loss)) in seen.iter_mut().enumerate() {
            let g = &world.varieties[i].grammar;
            *ahead |= g.pronoun_case() == PronounCase::Ahead;
            *relic |= g.pronoun_case() == PronounCase::Relic;
            *loss |= !g.has_object_marking();
        }
    }
    for i in 0..3 {
        let original = &founded.varieties[i].grammar;
        let object = original.has_object_marking();
        let particle = original
            .markers
            .iter()
            .any(|m| m.category == Category::Object && m.kind == MarkerKind::Particle);
        let (ahead, relic, loss) = seen[i];
        let v = &history.latest().varieties[i];
        counts.record(v, object, particle, ahead, relic, loss);
        show(v, &format!("workbench language seed {}", [31, 52, 73][i]));
    }
    counts.print("workbench", 3);
    println!(
        "Workbench founders reported separately: related descendants are not independent band samples."
    );
}

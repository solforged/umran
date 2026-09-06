use clap::{Parser, Subcommand};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use langgen_core::{
    apply_contact, conservative, radical, Aesthetic, BranchSpec, ContactEvent, Family, Language,
    Transfer,
};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, TableState, Tabs, Wrap,
};
use ratatui::{DefaultTerminal, Frame};
use std::io;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "langgen", about = "Seeded language generator")]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Interactive browser (default)
    Tui {
        #[arg(short, long, default_value = "elvish")]
        aesthetic: String,
        #[arg(short, long, default_value_t = 42)]
        seed: u64,
    },
    /// Print samples to stdout
    Gen {
        #[arg(short, long, default_value = "elvish")]
        aesthetic: String,
        #[arg(short, long, default_value_t = 42)]
        seed: u64,
        #[arg(short, long, default_value_t = 12)]
        count: usize,
        #[arg(long)]
        json: bool,
    },
    /// List built-in aesthetic packs
    Packs,
    /// Print a proto language and conservative/radical daughters
    Family {
        #[arg(short, long, default_value = "elvish")]
        aesthetic: String,
        #[arg(short, long, default_value_t = 42)]
        seed: u64,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.cmd {
        None => run_tui("elvish", 42)?,
        Some(Cmd::Tui { aesthetic, seed }) => run_tui(&aesthetic, seed)?,
        Some(Cmd::Packs) => {
            for a in Aesthetic::all() {
                println!("{:<10} {}", a.id, a.description);
            }
        }
        Some(Cmd::Gen {
            aesthetic,
            seed,
            count,
            json,
        }) => {
            let pack = Aesthetic::by_id(&aesthetic)
                .ok_or_else(|| format!("unknown aesthetic '{aesthetic}'; try `langgen packs`"))?;
            let lang = Language::new(seed, pack);
            if json {
                println!("{}", serde_json::to_string_pretty(&lang.snapshot(count))?);
            } else {
                print_lang(&lang, count);
            }
        }
        Some(Cmd::Family { aesthetic, seed }) => {
            let pack = Aesthetic::by_id(&aesthetic)
                .ok_or_else(|| format!("unknown aesthetic '{aesthetic}'; try `langgen packs`"))?;
            let family = grow_demo_family(seed, pack);
            print_family(&family);
        }
    }
    Ok(())
}

fn print_lang(lang: &Language, n: usize) {
    println!(
        "# {}  seed={}  autonym={}",
        lang.aesthetic.id, lang.seed, lang.autonym
    );
    println!(
        "C: {}",
        lang.inventory.ipas(&lang.inventory.consonants).join(" ")
    );
    println!(
        "V: {}",
        lang.inventory.ipas(&lang.inventory.vowels).join(" ")
    );
    println!("\n## people");
    for s in lang.sample_people(n) {
        println!("{s}");
    }
    println!("\n## places");
    for s in lang.sample_places(n) {
        println!("{s}");
    }
    println!("\n## words");
    for s in lang.sample_words(n) {
        println!("{s}");
    }
}

fn print_family(family: &Family) {
    println!(
        "# family seed={} proto={} autonym={}",
        family.seed, family.proto.aesthetic.id, family.proto.autonym
    );
    println!("## proto");
    for root in &family.roots {
        println!("{}  {}", root.id, family.proto.romanize(&root.proto));
    }
    for branch in &family.branches {
        println!("## {} autonym={}", branch.id, branch.language.autonym);
        for (id, word) in &branch.cognates {
            println!("{}  {}", id, branch.language.romanize(word));
        }
    }
}
fn grow_demo_family(seed: u64, pack: Aesthetic) -> Family {
    Family::grow(
        seed,
        pack.clone(),
        &[
            BranchSpec {
                id: "conservative",
                parent: None,
                aesthetic: pack.clone(),
                changes: conservative(),
            },
            BranchSpec {
                id: "radical",
                parent: None,
                aesthetic: pack,
                changes: radical(),
            },
        ],
    )
}

fn branch_form(family: &Family, branch_id: &str, gloss: &str) -> String {
    family
        .branches
        .iter()
        .find(|b| b.id == branch_id)
        .and_then(|b| {
            b.cognates
                .iter()
                .find(|(id, _)| id == gloss)
                .map(|(_, w)| b.language.romanize(w))
        })
        .unwrap_or_else(|| "—".into())
}

fn proto_form(family: &Family, gloss: &str) -> String {
    family
        .roots
        .iter()
        .find(|r| r.id == gloss)
        .map(|r| family.proto.romanize(&r.proto))
        .unwrap_or_else(|| "—".into())
}

fn cognate_ids(family: &Family) -> Vec<String> {
    let mut ids: Vec<String> = family.roots.iter().map(|r| r.id.to_string()).collect();
    for branch in &family.branches {
        for (id, _) in &branch.cognates {
            if !ids.iter().any(|have| have == id) {
                ids.push(id.clone());
            }
        }
    }
    ids
}

fn cognate_rows(family: &Family) -> Vec<(String, String, String, String)> {
    cognate_ids(family)
        .into_iter()
        .map(|id| {
            let proto = proto_form(family, &id);
            let cons = branch_form(family, "conservative", &id);
            let rad = branch_form(family, "radical", &id);
            (id, proto, cons, rad)
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Language,
    Family,
}

struct App {
    seed: u64,
    idx: usize,
    packs: Vec<Aesthetic>,
    lang: Language,
    family: Family,
    view: View,
    scroll: usize,
    status: String,
}

impl App {
    fn new(aesthetic: &str, seed: u64) -> Self {
        let packs = Aesthetic::all();
        let idx = packs.iter().position(|a| a.id == aesthetic).unwrap_or(0);
        let pack = packs[idx].clone();
        Self {
            seed,
            idx,
            lang: Language::new(seed, pack.clone()),
            family: grow_demo_family(seed, pack),
            packs,
            view: View::Language,
            scroll: 0,
            status: lang_keys().into(),
        }
    }

    fn rebuild(&mut self) {
        let pack = self.packs[self.idx].clone();
        self.lang = Language::new(self.seed, pack.clone());
        self.family = grow_demo_family(self.seed, pack);
        self.scroll = 0;
        self.status = self.keys();
    }

    fn keys(&self) -> String {
        match self.view {
            View::Language => lang_keys().into(),
            View::Family => family_keys().into(),
        }
    }

    fn clamp_scroll(&mut self) {
        let n = cognate_ids(&self.family).len();
        if n == 0 {
            self.scroll = 0;
        } else if self.scroll >= n {
            self.scroll = n - 1;
        }
    }

    fn loan(&mut self, reverse: bool) {
        let (donor, recipient) = if reverse {
            ("radical", "conservative")
        } else {
            ("conservative", "radical")
        };
        let have = self
            .family
            .branches
            .iter()
            .find(|b| b.id == recipient)
            .map(|b| {
                b.cognates
                    .iter()
                    .filter(|(id, _)| id.starts_with("borrow:"))
                    .count()
            })
            .unwrap_or(0);
        let n = (have + 8).min(36);
        apply_contact(
            &mut self.family,
            &ContactEvent {
                donor_branch: donor.into(),
                recipient_branch: recipient.into(),
                intensity: 1.0,
                transfer: Transfer::Lexicon { n },
            },
        );
        self.clamp_scroll();
        self.status = format!("loan {donor} → {recipient}  n={n}");
    }

    fn pidgin(&mut self) {
        apply_contact(
            &mut self.family,
            &ContactEvent {
                donor_branch: "conservative".into(),
                recipient_branch: "radical".into(),
                intensity: 1.0,
                transfer: Transfer::Pidgin { lexicon_cap: 16 },
            },
        );
        self.clamp_scroll();
        self.status = "pidgin mix onto radical".into();
    }
}

fn lang_keys() -> &'static str {
    "f family   ←/→ pack   r reroll   [ ] seed   e export   q quit"
}

fn family_keys() -> &'static str {
    "f language   c loan→   C loan←   p pidgin   j/k scroll   r reroll   e export   q quit"
}

fn run_tui(aesthetic: &str, seed: u64) -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new(aesthetic, seed);
    let result = event_loop(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;
        if !event::poll(Duration::from_millis(200))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
            KeyCode::Char('f') => {
                app.view = match app.view {
                    View::Language => View::Family,
                    View::Family => View::Language,
                };
                app.status = app.keys();
            }
            KeyCode::Left | KeyCode::BackTab => {
                app.idx = (app.idx + app.packs.len() - 1) % app.packs.len();
                app.rebuild();
            }
            KeyCode::Right | KeyCode::Tab => {
                app.idx = (app.idx + 1) % app.packs.len();
                app.rebuild();
            }
            KeyCode::Char('r') => {
                app.seed = app.seed.wrapping_add(1_000_003);
                app.rebuild();
            }
            KeyCode::Char(']') => {
                app.seed = app.seed.wrapping_add(1);
                app.rebuild();
            }
            KeyCode::Char('[') => {
                app.seed = app.seed.wrapping_sub(1);
                app.rebuild();
            }
            KeyCode::Char('e') => export(app),
            KeyCode::Char('c') if app.view == View::Family => app.loan(false),
            KeyCode::Char('C') if app.view == View::Family => app.loan(true),
            KeyCode::Char('p') if app.view == View::Family => app.pidgin(),
            KeyCode::Char('j') | KeyCode::Down if app.view == View::Family => {
                let n = cognate_ids(&app.family).len();
                if n > 0 {
                    app.scroll = (app.scroll + 1).min(n - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up if app.view == View::Family => {
                app.scroll = app.scroll.saturating_sub(1);
            }
            _ => {}
        }
    }
}

fn export(app: &mut App) {
    match app.view {
        View::Language => {
            let name = format!("langgen-{}-{}.json", app.lang.aesthetic.id, app.seed);
            match serde_json::to_string_pretty(&app.lang.snapshot(24)) {
                Ok(body) => match std::fs::write(&name, body) {
                    Ok(()) => app.status = format!("wrote {name}"),
                    Err(e) => app.status = format!("export failed: {e}"),
                },
                Err(e) => app.status = format!("export failed: {e}"),
            }
        }
        View::Family => {
            let name = format!("langgen-family-{}-{}.json", app.packs[app.idx].id, app.seed);
            let cognates: Vec<serde_json::Value> = cognate_rows(&app.family)
                .into_iter()
                .map(|(id, proto, conservative, radical)| {
                    serde_json::json!({
                        "id": id,
                        "proto": proto,
                        "conservative": conservative,
                        "radical": radical,
                    })
                })
                .collect();
            let body = serde_json::json!({
                "seed": app.seed,
                "aesthetic": app.packs[app.idx].id,
                "proto_autonym": app.family.proto.autonym,
                "cognates": cognates,
            });
            match serde_json::to_string_pretty(&body) {
                Ok(text) => match std::fs::write(&name, text) {
                    Ok(()) => app.status = format!("wrote {name}"),
                    Err(e) => app.status = format!("export failed: {e}"),
                },
                Err(e) => app.status = format!("export failed: {e}"),
            }
        }
    }
}

fn accent(id: &str) -> Color {
    match id {
        "elvish" => Color::Cyan,
        "kuo-toa" => Color::Blue,
        "illithid" => Color::Magenta,
        _ => Color::Yellow,
    }
}

fn ui(f: &mut Frame, app: &App) {
    match app.view {
        View::Language => ui_language(f, app),
        View::Family => ui_family(f, app),
    }
}

fn chrome<'a>(
    f: &mut Frame,
    app: &'a App,
    color: Color,
    title_extra: String,
) -> ratatui::layout::Rect {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(f.area());

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " langgen ",
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(title_extra),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(color)),
    );
    f.render_widget(title, chunks[0]);

    let tabs: Vec<Line> = app
        .packs
        .iter()
        .map(|a| Line::from(a.name.clone()))
        .collect();
    let tabs = Tabs::new(tabs)
        .select(app.idx)
        .highlight_style(Style::default().fg(color).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).title("aesthetic"));
    f.render_widget(tabs, chunks[1]);

    let footer = Paragraph::new(app.status.clone())
        .block(Block::default().borders(Borders::ALL).title("keys"));
    f.render_widget(footer, chunks[3]);
    chunks[2]
}

fn ui_language(f: &mut Frame, app: &App) {
    let color = accent(&app.lang.aesthetic.id);
    let body = chrome(
        f,
        app,
        color,
        format!(" seed {}   autonym {} ", app.seed, app.lang.autonym),
    );

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(28),
            Constraint::Percentage(24),
            Constraint::Percentage(24),
            Constraint::Percentage(24),
        ])
        .split(body);

    let cons = app
        .lang
        .inventory
        .ipas(&app.lang.inventory.consonants)
        .join(" ");
    let vows = app
        .lang
        .inventory
        .ipas(&app.lang.inventory.vowels)
        .join(" ");
    let onsets = app.lang.generator.onset_ipas();
    let onset_line = onsets
        .iter()
        .take(18)
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    let inv = Paragraph::new(vec![
        Line::from(Span::styled("consonants", Style::default().fg(color))),
        Line::from(cons),
        Line::from(""),
        Line::from(Span::styled("vowels", Style::default().fg(color))),
        Line::from(vows),
        Line::from(""),
        Line::from(Span::styled("onsets", Style::default().fg(color))),
        Line::from(onset_line),
        Line::from(""),
        Line::from(app.lang.aesthetic.description.clone()),
    ])
    .wrap(Wrap { trim: true })
    .block(Block::default().borders(Borders::ALL).title("inventory"));
    f.render_widget(inv, cols[0]);

    f.render_widget(
        name_list("people", &app.lang.sample_people(16), color),
        cols[1],
    );
    f.render_widget(
        name_list("places", &app.lang.sample_places(16), color),
        cols[2],
    );
    f.render_widget(
        name_list("words", &app.lang.sample_words(16), color),
        cols[3],
    );
}

fn ui_family(f: &mut Frame, app: &App) {
    let color = accent(&app.packs[app.idx].id);
    let body = chrome(
        f,
        app,
        color,
        format!(
            " seed {}   family  proto {} ",
            app.seed, app.family.proto.autonym
        ),
    );

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(26), Constraint::Min(40)])
        .split(body);

    let mut lines = vec![
        Line::from(Span::styled("proto", Style::default().fg(color))),
        Line::from(app.family.proto.autonym.clone()),
        Line::from(""),
    ];
    for branch in &app.family.branches {
        lines.push(Line::from(Span::styled(
            branch.id.clone(),
            Style::default().fg(color),
        )));
        lines.push(Line::from(branch.language.autonym.clone()));
        let loans = branch
            .cognates
            .iter()
            .filter(|(id, _)| id.starts_with("borrow:"))
            .count();
        if loans > 0 {
            lines.push(Line::from(format!("{loans} loans")));
        }
        lines.push(Line::from(""));
    }
    f.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("branches")),
        cols[0],
    );

    let rows: Vec<Row> = cognate_rows(&app.family)
        .into_iter()
        .map(|(id, proto, cons, rad)| {
            let style = if id.starts_with("borrow:") {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(id),
                Cell::from(proto),
                Cell::from(cons),
                Cell::from(rad),
            ])
            .style(style)
        })
        .collect();
    let header = Row::new(["gloss", "proto", "cons.", "rad."])
        .style(Style::default().fg(color).add_modifier(Modifier::BOLD));
    let table = Table::new(
        rows,
        [
            Constraint::Length(14),
            Constraint::Min(12),
            Constraint::Min(12),
            Constraint::Min(12),
        ],
    )
    .header(header)
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .block(Block::default().borders(Borders::ALL).title("cognates"));
    let mut state = TableState::default();
    state.select(Some(app.scroll));
    f.render_stateful_widget(table, cols[1], &mut state);
}

fn name_list<'a>(title: &'a str, items: &'a [String], color: Color) -> List<'a> {
    let rows: Vec<ListItem> = items.iter().map(|s| ListItem::new(s.as_str())).collect();
    List::new(rows).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(color)),
    )
}

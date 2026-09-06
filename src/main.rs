use clap::{Parser, Subcommand};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use langgen_core::{Aesthetic, Language};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap};
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
            let pack = Aesthetic::by_id(&aesthetic).ok_or_else(|| {
                format!("unknown aesthetic '{aesthetic}'; try `langgen packs`")
            })?;
            let lang = Language::new(seed, pack);
            if json {
                println!("{}", serde_json::to_string_pretty(&lang.snapshot(count))?);
            } else {
                print_lang(&lang, count);
            }
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

struct App {
    seed: u64,
    idx: usize,
    packs: Vec<Aesthetic>,
    lang: Language,
    status: String,
}

impl App {
    fn new(aesthetic: &str, seed: u64) -> Self {
        let packs = Aesthetic::all();
        let idx = packs
            .iter()
            .position(|a| a.id == aesthetic)
            .unwrap_or(0);
        let lang = Language::new(seed, packs[idx].clone());
        Self {
            seed,
            idx,
            packs,
            lang,
            status: "←/→ pack   r reroll   [ ] seed   e export   q quit".into(),
        }
    }

    fn rebuild(&mut self) {
        self.lang = Language::new(self.seed, self.packs[self.idx].clone());
    }
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
            _ => {}
        }
    }
}

fn export(app: &mut App) {
    let name = format!("langgen-{}-{}.json", app.lang.aesthetic.id, app.seed);
    match serde_json::to_string_pretty(&app.lang.snapshot(24)) {
        Ok(body) => match std::fs::write(&name, body) {
            Ok(()) => app.status = format!("wrote {name}"),
            Err(e) => app.status = format!("export failed: {e}"),
        },
        Err(e) => app.status = format!("export failed: {e}"),
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
    let color = accent(&app.lang.aesthetic.id);
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
        Span::styled(" langgen ", Style::default().fg(color).add_modifier(Modifier::BOLD)),
        Span::raw(format!(
            " seed {}   autonym {} ",
            app.seed, app.lang.autonym
        )),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(color)));
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

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(28),
            Constraint::Percentage(24),
            Constraint::Percentage(24),
            Constraint::Percentage(24),
        ])
        .split(chunks[2]);

    let cons = app.lang.inventory.ipas(&app.lang.inventory.consonants).join(" ");
    let vows = app.lang.inventory.ipas(&app.lang.inventory.vowels).join(" ");
    let onsets = app.lang.generator.onset_ipas();
    let onset_line = onsets.iter().take(18).cloned().collect::<Vec<_>>().join(" ");
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
    f.render_widget(inv, body[0]);

    f.render_widget(name_list("people", &app.lang.sample_people(16), color), body[1]);
    f.render_widget(name_list("places", &app.lang.sample_places(16), color), body[2]);
    f.render_widget(name_list("words", &app.lang.sample_words(16), color), body[3]);

    let footer = Paragraph::new(app.status.clone())
        .block(Block::default().borders(Borders::ALL).title("keys"));
    f.render_widget(footer, chunks[3]);
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

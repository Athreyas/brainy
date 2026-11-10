// brainy v2 — keyboard-first TUI. No mouse needed.
// Top tabs, Cmd/Ctrl-K palette, vim nav, grouped tasks.

mod bookmarks;
mod tasks;
mod vault;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs},
    Frame, Terminal,
};
use std::io::stdout;
use vault::{Note, Vault};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Memory,
    Actions,
    Calendar,
    Search,
    Tasks,
    Bookmarks,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Tab::Memory => "Memory",
            Tab::Actions => "Actions",
            Tab::Calendar => "Calendar",
            Tab::Search => "Search",
            Tab::Tasks => "Tasks",
            Tab::Bookmarks => "Bookmarks",
        }
    }
    fn all() -> [Tab; 6] {
        [Tab::Memory, Tab::Actions, Tab::Calendar, Tab::Search, Tab::Tasks, Tab::Bookmarks]
    }
}

struct App {
    tab: Tab,
    vault: Vault,
    tasks: Vec<tasks::TaskItem>,
    bookmarks: Vec<bookmarks::Bookmark>,
    search_input: String,
    search_results: Vec<String>,
    cursor: usize,
    notice: String,
    task_cursor: usize,
    // command palette (⌘K)
    palette_open: bool,
    palette_input: String,
    palette_index: usize,
    // tasks: j/k + numbers for group jump (0=Overdue 1=Week 2=High 3=Later)
    task_filter: Option<usize>,
}

impl App {
    fn new() -> anyhow::Result<Self> {
        let root = vault::vault_path();
        let vault = vault::load_vault(&root).unwrap_or_default();
        let tasks = tasks::fetch_tasks().unwrap_or_default();
        let bookmarks = bookmarks::load_bookmarks();
        Ok(App {
            tab: Tab::Memory,
            vault,
            tasks,
            bookmarks,
            search_input: String::new(),
            search_results: Vec::new(),
            cursor: 0,
            notice: String::new(),
            task_cursor: 0,
            palette_open: false,
            palette_input: String::new(),
            palette_index: 0,
            task_filter: None,
        })
    }

    fn refresh(&mut self) {
        if let Ok(v) = vault::load_vault(&vault::vault_path()) {
            self.vault = v;
        }
        if let Ok(t) = tasks::fetch_tasks() {
            self.tasks = t;
        }
        self.bookmarks = bookmarks::load_bookmarks();
        self.notice = "refreshed".into();
    }

    fn run_search(&mut self) {
        if self.search_input.is_empty() {
            self.search_results.clear();
            return;
        }
        self.search_results = vault::search(&self.vault, &self.search_input);
        self.cursor = 0;
    }

    /// Flat list of (group label, task) in display order — buckets per user ask.
    /// A task appears in exactly one bucket, in priority order:
    /// Overdue > Due this week > High priority > Later.
    fn task_rows(&self) -> Vec<(String, tasks::TaskItem)> {
        let pending = tasks::pending_tasks(&self.tasks);
        let bucket = |t: &tasks::TaskItem| -> usize {
            if t.is_overdue() { 0 }
            else if t.is_due_this_week() { 1 }
            else if t.priority.as_deref() == Some("H") { 2 }
            else { 3 }
        };
        let labels = ["Overdue", "Due this week", "High priority", "Later"];
        let mut rows: Vec<(String, tasks::TaskItem)> = Vec::new();
        for gi in 0..4 {
            if let Some(f) = self.task_filter {
                if f != gi { continue; }
            }
            for t in pending.iter().filter(|t| bucket(t) == gi) {
                rows.push((labels[gi].to_string(), t.clone()));
            }
        }
        rows
    }
}

/// Palette entries: (title, action description)
fn palette_entries() -> Vec<(String, &'static str)> {
    vec![
        ("Go to Memory".into(), "tab-m"),
        ("Go to Actions".into(), "tab-a"),
        ("Go to Calendar".into(), "tab-c"),
        ("Go to Search".into(), "tab-s"),
        ("Go to Tasks".into(), "tab-t"),
        ("Go to Bookmarks".into(), "tab-b"),
        ("Refresh all".into(), "R"),
        ("Quit".into(), "Q"),
    ]
}

fn run_palette(app: &mut App, idx: usize) {
    match idx {
        0 => app.tab = Tab::Memory,
        1 => app.tab = Tab::Actions,
        2 => app.tab = Tab::Calendar,
        3 => app.tab = Tab::Search,
        4 => app.tab = Tab::Tasks,
        5 => app.tab = Tab::Bookmarks,
        6 => app.refresh(),
        7 => std::process::exit(0),
        _ => {}
    }
}

fn filtered_palette(app: &App) -> Vec<(String, &'static str)> {
    let q = app.palette_input.to_lowercase();
    palette_entries().into_iter().filter(|(t, _)| t.to_lowercase().contains(&q)).collect()
}

fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new()?;
    let result = run_app(&mut terminal, &mut app);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;
        if let Event::Key(k) = event::read()? {
            if handle_key(app, k.code, k.modifiers)? {
                break;
            }
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, code: KeyCode, mods: KeyModifiers) -> anyhow::Result<bool> {
    // ⌘K / Ctrl-K opens the palette from anywhere
    if (mods.contains(KeyModifiers::SUPER) || mods.contains(KeyModifiers::CONTROL))
        && matches!(code, KeyCode::Char('k') | KeyCode::Char('K'))
    {
        app.palette_open = !app.palette_open;
        app.palette_input.clear();
        app.palette_index = 0;
        return Ok(false);
    }
    if app.palette_open {
        match code {
            KeyCode::Esc => { app.palette_open = false; }
            KeyCode::Backspace => { app.palette_input.pop(); app.palette_index = 0; }
            KeyCode::Up => { app.palette_index = app.palette_index.saturating_sub(1); }
            KeyCode::Down => {
                let n = filtered_palette(app).len();
                if app.palette_index + 1 < n { app.palette_index += 1; }
            }
            KeyCode::Enter => {
                let entries = filtered_palette(app);
                if let Some((_, hint)) = entries.get(app.palette_index) {
                    let hint = *hint;
                    let entries = palette_entries();
                    if let Some(idx) = entries.iter().position(|(t, h)| *h == hint && (app.palette_input.is_empty() || t.to_lowercase().contains(&app.palette_input.to_lowercase()))) {
                        run_palette(app, idx);
                    }
                }
                app.palette_open = false;
            }
            KeyCode::Char(c) => { app.palette_input.push(c); app.palette_index = 0; }
            _ => {}
        }
        return Ok(false);
    }

    // global keys (not when typing in Search)
    let in_search = app.tab == Tab::Search;
    if !in_search {
        match code {
            KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(true),
            KeyCode::Char('c') if mods.contains(KeyModifiers::CONTROL) => return Ok(true),
            KeyCode::Char('R') if mods.is_empty() => { app.refresh(); }
            // number tabs: 1-6
            KeyCode::Char(n @ '1'..='6') => {
                let idx = (n as usize) - ('1' as usize);
                let all = Tab::all();
                if idx < all.len() { app.tab = all[idx]; }
            }
            KeyCode::Char('m') => app.tab = Tab::Memory,
            KeyCode::Char('a') => app.tab = Tab::Actions,
            KeyCode::Char('d') => app.tab = Tab::Calendar,
            KeyCode::Char('s') => app.tab = Tab::Search,
            KeyCode::Char('t') => app.tab = Tab::Tasks,
            KeyCode::Char('b') => app.tab = Tab::Bookmarks,
            KeyCode::Left => {
                let all = Tab::all();
                let i = all.iter().position(|t| *t == app.tab).unwrap_or(0);
                app.tab = all[(i + all.len() - 1) % all.len()];
            }
            KeyCode::Right => {
                let all = Tab::all();
                let i = all.iter().position(|t| *t == app.tab).unwrap_or(0);
                app.tab = all[(i + 1) % all.len()];
            }
            KeyCode::Char('h') => {
                let all = Tab::all();
                let i = all.iter().position(|t| *t == app.tab).unwrap_or(0);
                app.tab = all[(i + all.len() - 1) % all.len()];
            }
            KeyCode::Char('l') => {
                let all = Tab::all();
                let i = all.iter().position(|t| *t == app.tab).unwrap_or(0);
                app.tab = all[(i + 1) % all.len()];
            }
            _ => {}
        }
    }

    match app.tab {
        Tab::Search => match code {
            KeyCode::Backspace => {
                app.search_input.pop();
                app.run_search();
            }
            KeyCode::Char(c) => {
                app.search_input.push(c);
                app.run_search();
            }
            KeyCode::Esc => { app.tab = Tab::Memory; }
            _ => {}
        },
        Tab::Tasks => match code {
            KeyCode::Up | KeyCode::Char('k') if app.task_cursor > 0 => app.task_cursor -= 1,
            KeyCode::Down | KeyCode::Char('j') => app.task_cursor += 1,
            KeyCode::Char('x') | KeyCode::Enter => {
                let rows = app.task_rows();
                if let Some((_, t)) = rows.get(app.task_cursor) {
                    match tasks::complete_task(t.id) {
                        Ok(()) => {
                            app.notice = format!("done: {}", t.description);
                            app.refresh();
                        }
                        Err(e) => app.notice = format!("error: {e:#}"),
                    }
                }
            }
            KeyCode::Char(n @ '0'..='3') => {
                let gi = (n as usize) - ('0' as usize);
                app.task_filter = Some(gi);
                app.task_cursor = 0;
            }
            KeyCode::Char('f') => {
                app.task_filter = None;
                app.task_cursor = 0;
            }
            _ => {}
        },
        Tab::Memory | Tab::Actions | Tab::Bookmarks => match code {
            KeyCode::Up | KeyCode::Char('k') if app.cursor > 0 => app.cursor -= 1,
            KeyCode::Down | KeyCode::Char('j') => app.cursor += 1,
            _ => {}
        },
        Tab::Calendar => {}
    }
    Ok(false)
}

fn ui(f: &mut Frame, app: &mut App) {
    // ---- top bar: title + tabs + palette hint ----
    let top = Layout::vertical([Constraint::Length(1), Constraint::Length(3), Constraint::Min(10), Constraint::Length(1), Constraint::Length(1)])
        .split(f.area());
    // title row
    let title = Line::from(vec![
        Span::styled(" brainy ", Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(format!("  {} tabs · ⌘K command palette · q quit", Tab::all().len())),
    ]);
    f.render_widget(Paragraph::new(title), top[0]);

    let tab_titles: Vec<Line> = Tab::all()
        .iter()
        .map(|t| Line::from(vec![Span::raw(format!(" {} ", t.label()))]))
        .collect();
    let tab_index = Tab::all().iter().position(|t| *t == app.tab).unwrap_or(0);
    let tabs = Tabs::new(tab_titles)
        .select(tab_index)
        .block(Block::default().borders(Borders::ALL).title(" tabs — h/l or ←/→ "))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, top[1]);

    match app.tab {
        Tab::Memory => draw_memory(f, app, top[2]),
        Tab::Actions => draw_actions(f, app, top[2]),
        Tab::Calendar => draw_calendar(f, app, top[2]),
        Tab::Search => draw_search(f, app, top[2]),
        Tab::Tasks => draw_tasks(f, app, top[2]),
        Tab::Bookmarks => draw_bookmarks(f, app, top[2]),
    }

    let drive = format!(
        "⏺ vault · {} notes · {} pending · {} bookmarks ({})",
        app.vault.notes.len(),
        tasks::pending_tasks(&app.tasks).len(),
        app.bookmarks.len(),
        bookmarks::by_browser(&app.bookmarks)
            .iter()
            .map(|(b, n)| format!("{b}:{n}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    f.render_widget(
        Paragraph::new(drive).style(Style::default().fg(Color::Cyan)),
        top[3],
    );

    let hint = if app.notice.is_empty() {
        "h/l switch tab · 1-6 jump · j/k move · x/⏎ complete · 0-3 task group · f all groups · R refresh · ⌘K palette · q quit".to_string()
    } else {
        app.notice.clone()
    };
    f.render_widget(
        Paragraph::new(hint).style(Style::default().fg(Color::DarkGray)),
        top[4],
    );

    if app.palette_open {
        draw_palette(f, app);
    }
}

fn draw_palette(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 35, f.area());
    f.render_widget(Clear, area);
    let block = Block::default().borders(Borders::ALL).title(format!(" ⌘K — {}\" ", app.palette_input));
    let entries = filtered_palette(app);
    let mut lines = vec![Line::from(Span::styled("type to filter · ↑/↓ · ⏎ run · esc close", Style::default().fg(Color::DarkGray)))];
    for (i, (title, hint)) in entries.iter().take(10).enumerate() {
        let style = if i == app.palette_index {
            Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {title}"), style),
            Span::styled(format!("   [{hint}]"), Style::default().fg(Color::DarkGray)),
        ]));
    }
    if entries.is_empty() {
        lines.push(Line::from("  (no matches)"));
    }
    f.render_widget(Paragraph::new(lines).block(block), area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);
    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}

fn draw_memory(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    // Memory folder becomes the "knowledge base": sessions + atomic facts,
    // collapsed into section lines rather than 50 individual files.
    let memory: Vec<&Note> = app
        .vault
        .notes
        .iter()
        .filter(|n| n.rel_path.starts_with("06-Memory/"))
        .collect();
    let sessions: Vec<_> = memory
        .iter()
        .filter(|n| n.rel_path.contains("/sessions/"))
        .collect();
    let facts: Vec<_> = memory
        .iter()
        .filter(|n| n.rel_path.contains("/memory/"))
        .collect();

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        format!("Memory — {} files grouped by kind", memory.len()),
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("Session logs ({}) — durable context", sessions.len()),
        Style::default().fg(Color::Yellow),
    )));
    for n in sessions.iter().take(12) {
        lines.push(Line::from(format!("  ▸ {}  ·  {}", n.title, n.folder)));
    }
    if sessions.len() > 12 {
        lines.push(Line::from(format!("  … and {} more", sessions.len() - 12)));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("Atomic facts ({}) — the curated memory", facts.len()),
        Style::default().fg(Color::Yellow),
    )));
    for n in facts.iter().take(12) {
        lines.push(Line::from(format!("  ▸ {}  ·  {}", n.title, n.folder)));
    }
    if facts.len() > 12 {
        lines.push(Line::from(format!("  … and {} more", facts.len() - 12)));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(format!(
        "Vault overview: {} folders · {} notes total",
        app.vault.folders.len(),
        app.vault.notes.len()
    )));
    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Memory")),
        area,
    );
}

fn draw_actions(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    // Curated: only high-signal sections — Daily/Weekly/Project checkboxes.
    // Inbox copies and truncated "]" fragments are skipped (they pollute the list).
    let actions = vault::all_open_actions(&app.vault);
    let mut lines: Vec<Line> = vec![Line::from(Span::styled(
        format!("Open checkboxes ({}) — 09-Planning, 01-Projects, 03-Learning only", actions.len()),
        Style::default().add_modifier(Modifier::BOLD),
    ))];
    for (note, c) in actions.iter().take(60) {
        lines.push(Line::from(vec![
            Span::styled("  ☐ ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{}  ", c.text)),
            Span::styled(format!("→ {note}"), Style::default().fg(Color::DarkGray)),
        ]));
    }
    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Actions — j/k move · x complete")),
        area,
    );
}

fn draw_calendar(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let activity = vault::calendar_activity(&app.vault);
    let mut lines: Vec<Line> = vec![];
    lines.push(Line::from(Span::styled(
        "Daily notes — your memory calendar",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    // compact month strip
    let days = vault::current_month_days();
    let mut strip = String::from("   ");
    for d in &days {
        if let Some(date) = d {
            let has = activity.iter().any(|(a, _)| a == date);
            strip.push_str(&format!("{} ", if has { "●" } else { "·" }));
        }
    }
    lines.push(Line::from(strip));
    lines.push(Line::from("   1                       31".to_string()));
    lines.push(Line::from(""));
    for (date, title) in activity.iter().rev().take(14) {
        lines.push(Line::from(format!("  {}  {}", vault::fmt_date(date), title)));
    }
    if activity.is_empty() {
        lines.push(Line::from("  (no daily notes found)"));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Weekly plans",
        Style::default().fg(Color::Yellow),
    )));
    for n in &app.vault.weekly_notes {
        lines.push(Line::from(format!("  ▸ {}", n.title)));
    }
    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Calendar")),
        area,
    );
}

fn draw_search(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(5)]).split(area);
    let input = Paragraph::new(app.search_input.clone())
        .block(Block::default().borders(Borders::ALL).title(" Search (type to filter all notes) "));
    f.render_widget(input, chunks[0]);

    let mut lines: Vec<Line> = Vec::new();
    if app.search_input.is_empty() {
        lines.push(Line::from("Start typing — searches titles, paths, tags, body text."));
    } else {
        lines.push(Line::from(Span::styled(
            format!("{} matches", app.search_results.len()),
            Style::default().add_modifier(Modifier::BOLD),
        )));
        for r in app.search_results.iter().take(50) {
            lines.push(Line::from(format!("  ▸ {r}")));
        }
    }
    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL)),
        chunks[1],
    );
}

fn draw_bookmarks(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let groups = bookmarks::grouped(&app.bookmarks);
    let mut lines: Vec<Line> = vec![];
    lines.push(Line::from(Span::styled(
        format!(
            "{} bookmarks from Dia & friends, smart-categorized",
            app.bookmarks.len()
        ),
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    let mut shown = 0usize;
    for (cat, items) in &groups {
        lines.push(Line::from(Span::styled(
            format!("{} ({})", cat, items.len()),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
        for b in items.iter().take(8) {
            lines.push(Line::from(vec![
                Span::raw("  ▸ "),
                Span::styled(b.name.clone(), Style::default().fg(Color::White)),
                Span::styled(
                    format!(" · {}", bookmarks::describe(b)),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
            shown += 1;
            if shown > 120 {
                break;
            }
        }
        lines.push(Line::from(""));
    }
    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Bookmarks")),
        area,
    );
}

fn draw_tasks(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let stats = tasks::stats(&app.tasks);
    let rows = app.task_rows();
    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            format!(
                "{} pending · {} overdue · {} due this week · {} high priority · {} total",
                stats.pending, stats.overdue, stats.due_this_week, stats.high_prio, stats.total
            ),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "j/k move · x/⏎ complete · 0 Overdue 1 Week 2 High 3 Later · f show all · R refresh",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
    ];
    let mut last_label = String::new();
    for (i, (label, t)) in rows.iter().take(60).enumerate() {
        if label != &last_label {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("─ {} ─", label),
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            )));
            last_label = label.clone();
        }
        let cursor = if i == app.task_cursor { "▸ " } else { "  " };
        let style = if i == app.task_cursor {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else {
            Style::default()
        };
        let due = t
            .due_date()
            .map(|d| vault::fmt_date(&d))
            .unwrap_or_else(|| "—".into());
        lines.push(Line::from(vec![
            Span::styled(cursor, style),
            Span::styled("☐ ", style),
            Span::styled(t.description.clone(), style),
            Span::raw(format!(
                "   [{}] due:{} urg:{:.1}",
                if t.project.is_empty() { "inbox" } else { &t.project },
                due,
                t.urgency
            )),
        ]));
    }
    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Tasks")),
        area,
    );
}

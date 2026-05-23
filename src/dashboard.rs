use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::layout::{Constraint, Direction, Layout, Margin};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::scrollbar;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, Cell, Gauge, List, ListItem, ListState, Paragraph, Row, Scrollbar,
    ScrollbarOrientation, ScrollbarState, Table, TableState,
};
use ratatui::{Frame, Terminal};
use std::io::{stdout};
use std::path::Path;
use std::time::{Duration, Instant};

/// Data snapshot fetched from the two SQLite databases.
pub struct DashboardData {
    pub tasks: Vec<TaskRow>,
    pub agents: Vec<AgentRow>,
    pub letters: Vec<LetterRow>,
    pub books_count: u64,
    pub concepts_count: u64,
    pub chunks_count: u64,
    pub links_count: u64,
}

#[derive(Clone)]
pub struct TaskRow {
    pub id: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub agent_count: usize,
}

#[derive(Clone)]
pub struct AgentRow {
    pub task_id: String,
    pub persona_id: String,
    pub personality_id: String,
    pub mood: String,
    pub assigned_at: String,
}

#[derive(Clone)]
pub struct LetterRow {
    pub task_id: String,
    pub from_persona: String,
    pub to_persona: Option<String>,
    pub content: String,
    pub mood_at_send: String,
    pub sent_at: String,
}

impl DashboardData {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            agents: Vec::new(),
            letters: Vec::new(),
            books_count: 0,
            concepts_count: 0,
            chunks_count: 0,
            links_count: 0,
        }
    }
}

/// The TUI application state.
pub struct DashboardApp {
    pub data: DashboardData,
    pub tasks_state: ListState,
    pub agents_state: TableState,
    pub letters_scroll: usize,
    pub should_quit: bool,
    pub last_refresh: Instant,
    pub refresh_interval: Duration,
    pub tasks_db_path: String,
    pub knowledge_db_path: String,
}

impl DashboardApp {
    pub fn new(tasks_db_path: &str, knowledge_db_path: &str) -> Result<Self> {
        let mut app = Self {
            data: DashboardData::new(),
            tasks_state: ListState::default(),
            agents_state: TableState::default(),
            letters_scroll: 0,
            should_quit: false,
            last_refresh: Instant::now() - Duration::from_secs(60),
            refresh_interval: Duration::from_secs(5),
            tasks_db_path: tasks_db_path.to_string(),
            knowledge_db_path: knowledge_db_path.to_string(),
        };
        app.refresh_data()?;
        if !app.data.tasks.is_empty() {
            app.tasks_state.select(Some(0));
        }
        if !app.data.agents.is_empty() {
            app.agents_state.select(Some(0));
        }
        Ok(app)
    }

    /// Query both SQLite databases and rebuild the data snapshot.
    pub fn refresh_data(&mut self) -> Result<()> {
        use rusqlite::Connection;

        let mut data = DashboardData::new();

        // --- swarm_tasks.db ---
        if Path::new(&self.tasks_db_path).exists() {
            let conn = Connection::open(&self.tasks_db_path)
                .with_context(|| format!("Failed to open tasks DB at {}", self.tasks_db_path))?;

            // Tasks
            let mut stmt = conn.prepare(
                "SELECT id, name, status, created_at FROM tasks ORDER BY created_at DESC"
            )?;
            let task_rows = stmt.query_map([], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let status: String = row.get(2)?;
                let created_at: String = row.get(3)?;

                // Count agents for this task
                let mut count_stmt = conn.prepare(
                    "SELECT COUNT(*) FROM task_agents WHERE task_id = ?1"
                ).ok();
                let agent_count = count_stmt
                    .as_mut()
                    .and_then(|s| s.query_row([&id], |r| r.get::<_, i64>(0)).ok())
                    .unwrap_or(0) as usize;

                Ok(TaskRow {
                    id,
                    name,
                    status,
                    created_at,
                    agent_count,
                })
            })?;
            for row in task_rows {
                if let Ok(r) = row {
                    data.tasks.push(r);
                }
            }

            // Agents (all)
            let mut stmt = conn.prepare(
                "SELECT task_id, persona_id, personality_id, mood, assigned_at
                 FROM task_agents ORDER BY assigned_at DESC"
            )?;
            let agent_rows = stmt.query_map([], |row| {
                Ok(AgentRow {
                    task_id: row.get(0)?,
                    persona_id: row.get(1)?,
                    personality_id: row.get(2)?,
                    mood: row.get(3)?,
                    assigned_at: row.get(4)?,
                })
            })?;
            for row in agent_rows {
                if let Ok(r) = row {
                    data.agents.push(r);
                }
            }

            // Letters (latest 50)
            let mut stmt = conn.prepare(
                "SELECT task_id, from_persona, to_persona, content, mood_at_send, sent_at
                 FROM letters ORDER BY sent_at DESC LIMIT 50"
            )?;
            let letter_rows = stmt.query_map([], |row| {
                Ok(LetterRow {
                    task_id: row.get(0)?,
                    from_persona: row.get(1)?,
                    to_persona: row.get(2)?,
                    content: row.get(3)?,
                    mood_at_send: row.get(4)?,
                    sent_at: row.get(5)?,
                })
            })?;
            for row in letter_rows {
                if let Ok(r) = row {
                    data.letters.push(r);
                }
            }
        }

        // --- swarm_knowledge.db ---
        if Path::new(&self.knowledge_db_path).exists() {
            let conn = Connection::open(&self.knowledge_db_path)
                .with_context(|| format!("Failed to open knowledge DB at {}", self.knowledge_db_path))?;

            data.books_count = conn
                .query_row("SELECT COUNT(*) FROM books", [], |r| r.get::<_, i64>(0))
                .unwrap_or(0) as u64;

            data.concepts_count = conn
                .query_row("SELECT COUNT(*) FROM concepts", [], |r| r.get::<_, i64>(0))
                .unwrap_or(0) as u64;

            data.chunks_count = conn
                .query_row("SELECT COUNT(*) FROM chunks", [], |r| r.get::<_, i64>(0))
                .unwrap_or(0) as u64;

            data.links_count = conn
                .query_row("SELECT COUNT(*) FROM syntopical_links", [], |r| r.get::<_, i64>(0))
                .unwrap_or(0) as u64;
        }

        self.data = data;
        self.last_refresh = Instant::now();
        Ok(())
    }

    pub fn on_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Down | KeyCode::Char('j') => self.next_task(),
            KeyCode::Up | KeyCode::Char('k') => self.prev_task(),
            KeyCode::Right | KeyCode::Char('l') => self.next_agent(),
            KeyCode::Left | KeyCode::Char('h') => self.prev_agent(),
            KeyCode::PageDown => self.scroll_letters_down(),
            KeyCode::PageUp => self.scroll_letters_up(),
            KeyCode::Char('r') => {
                let _ = self.refresh_data();
            }
            _ => {}
        }
    }

    fn next_task(&mut self) {
        let i = match self.tasks_state.selected() {
            Some(i) if i + 1 < self.data.tasks.len() => i + 1,
            Some(_) => 0,
            None if !self.data.tasks.is_empty() => 0,
            None => return,
        };
        self.tasks_state.select(Some(i));
    }

    fn prev_task(&mut self) {
        let i = match self.tasks_state.selected() {
            Some(0) => self.data.tasks.len().saturating_sub(1),
            Some(i) => i - 1,
            None if !self.data.tasks.is_empty() => 0,
            None => return,
        };
        self.tasks_state.select(Some(i));
    }

    fn next_agent(&mut self) {
        let i = match self.agents_state.selected() {
            Some(i) if i + 1 < self.data.agents.len() => i + 1,
            Some(_) => 0,
            None if !self.data.agents.is_empty() => 0,
            None => return,
        };
        self.agents_state.select(Some(i));
    }

    fn prev_agent(&mut self) {
        let i = match self.agents_state.selected() {
            Some(0) => self.data.agents.len().saturating_sub(1),
            Some(i) => i - 1,
            None if !self.data.agents.is_empty() => 0,
            None => return,
        };
        self.agents_state.select(Some(i));
    }

    fn scroll_letters_down(&mut self) {
        self.letters_scroll = self.letters_scroll.saturating_add(3);
    }

    fn scroll_letters_up(&mut self) {
        self.letters_scroll = self.letters_scroll.saturating_sub(3);
    }
}

/// Run the dashboard TUI until the user quits.
pub fn run_dashboard(tasks_db_path: &str, knowledge_db_path: &str) -> Result<()> {
    let mut stdout = stdout();
    stdout.execute(EnterAlternateScreen)?;
    enable_raw_mode()?;

    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))?;

    let mut app = DashboardApp::new(tasks_db_path, knowledge_db_path)?;

    let tick_rate = Duration::from_millis(200);
    let mut last_tick = Instant::now();

    while !app.should_quit {
        // Draw
        terminal.draw(|f| draw(f, &mut app))?;

        // Poll events with timeout so we can refresh periodically
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.on_key(key.code);
                }
            }
        }

        // Periodic refresh
        if last_tick.elapsed() >= tick_rate {
            if app.last_refresh.elapsed() >= app.refresh_interval {
                let _ = app.refresh_data();
            }
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn draw(frame: &mut Frame, app: &mut DashboardApp) {
    let area = frame.area();

    // Overall layout: top bar, main body (2 cols), bottom bar
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // title bar
            Constraint::Min(8),  // body
            Constraint::Length(1), // status bar
        ])
        .split(area);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(main_layout[1]);

    let left_col = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(body[0]);

    let right_col = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(body[1]);

    // ── Title bar ──
    let title_text = format!(
        " OpenClaw Swarm Dashboard  |  Tasks: {}  Agents: {}  Books: {}  Concepts: {}  Chunks: {}  Links: {} ",
        app.data.tasks.len(),
        app.data.agents.len(),
        app.data.books_count,
        app.data.concepts_count,
        app.data.chunks_count,
        app.data.links_count
    );
    let title = Paragraph::new(title_text)
        .block(Block::default().borders(Borders::ALL).title("Swarm"))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    frame.render_widget(title, main_layout[0]);

    // ── Left col top: Tasks (List) ──
    let task_items: Vec<ListItem> = app
        .data
        .tasks
        .iter()
        .map(|t| {
            let status_color = match t.status.as_str() {
                "active" | "started" => Color::Green,
                "pending" | "created" => Color::Yellow,
                "completed" | "shipped" => Color::Blue,
                "failed" => Color::Red,
                _ => Color::Gray,
            };
            let line = Line::from(vec![
                Span::styled(
                    format!("[{}] ", t.status.to_uppercase()),
                    Style::default().fg(status_color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("{} ", t.name)),
                Span::styled(
                    format!("({} agents)", t.agent_count),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let tasks_list = List::new(task_items)
        .block(Block::default().borders(Borders::ALL).title("Tasks"))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");
    frame.render_stateful_widget(tasks_list, left_col[0], &mut app.tasks_state);

    // ── Left col bottom: Knowledge Stats (Gauge + Paragraph) ──
    let knowledge_block = Block::default().borders(Borders::ALL).title("Knowledge Graph");
    let inner = knowledge_block.inner(left_col[1]);
    frame.render_widget(knowledge_block, left_col[1]);

    let knowledge_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // books gauge
            Constraint::Length(3), // concepts gauge
            Constraint::Length(3), // chunks gauge
            Constraint::Length(3), // links gauge
            Constraint::Min(0),    // remaining
        ])
        .split(inner);

    // Normalize gauges against soft caps so they look good even with small counts
    let soft_cap = |n: u64| if n < 100 { 100 } else { n };

    let books_gauge = Gauge::default()
        .block(Block::default().title("Books").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Blue))
        .ratio(
            (app.data.books_count as f64 / soft_cap(app.data.books_count).max(1) as f64).min(1.0),
        )
        .label(format!("{}", app.data.books_count));
    frame.render_widget(books_gauge, knowledge_layout[0]);

    let concepts_gauge = Gauge::default()
        .block(Block::default().title("Concepts").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Magenta))
        .ratio(
            (app.data.concepts_count as f64 / soft_cap(app.data.concepts_count).max(1) as f64)
                .min(1.0),
        )
        .label(format!("{}", app.data.concepts_count));
    frame.render_widget(concepts_gauge, knowledge_layout[1]);

    let chunks_gauge = Gauge::default()
        .block(Block::default().title("Chunks").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Yellow))
        .ratio(
            (app.data.chunks_count as f64 / soft_cap(app.data.chunks_count).max(1) as f64).min(1.0),
        )
        .label(format!("{}", app.data.chunks_count));
    frame.render_widget(chunks_gauge, knowledge_layout[2]);

    let links_gauge = Gauge::default()
        .block(Block::default().title("Syntopical Links").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Green))
        .ratio(
            (app.data.links_count as f64 / soft_cap(app.data.links_count).max(1) as f64).min(1.0),
        )
        .label(format!("{}", app.data.links_count));
    frame.render_widget(links_gauge, knowledge_layout[3]);

    // ── Right col top: Agents (Table) ──
    let header = Row::new(vec!["Persona", "Personality", "Mood", "Task", "Assigned"])
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .bottom_margin(0);

    let agent_rows: Vec<Row> = app
        .data
        .agents
        .iter()
        .map(|a| {
            let mood_color = match a.mood.as_str() {
                "focused" | "calm" | "confident" => Color::Green,
                "excited" | "energetic" => Color::Cyan,
                "reflective" | "neutral" => Color::Yellow,
                "stressed" | "tired" | "frustrated" => Color::Red,
                _ => Color::Gray,
            };
            Row::new(vec![
                Cell::from(a.persona_id.clone()),
                Cell::from(a.personality_id.clone()),
                Cell::from(Span::styled(a.mood.clone(), Style::default().fg(mood_color))),
                Cell::from(a.task_id[..a.task_id.len().min(8)].to_string()),
                Cell::from(a.assigned_at[..a.assigned_at.len().min(16)].to_string()),
            ])
        })
        .collect();

    let agents_table = Table::new(agent_rows, [
        Constraint::Length(14),
        Constraint::Length(14),
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Length(16),
    ])
    .header(header)
    .block(Block::default().borders(Borders::ALL).title("Agents"))
    .row_highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));

    frame.render_stateful_widget(agents_table, right_col[0], &mut app.agents_state);

    // ── Right col bottom: Runner / Letters (Paragraph with scroll) ──
    let selected_task = app
        .tasks_state
        .selected()
        .and_then(|i| app.data.tasks.get(i))
        .cloned();

    let mut lines = vec![];

    // Show filtered letters for selected task if any
    if let Some(ref task) = selected_task {
        lines.push(Line::from(vec![
            Span::styled(
                format!("Task: {}  ", task.name),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("[{}]", task.status),
                Style::default().fg(Color::Yellow),
            ),
        ]));
        lines.push(Line::from(""));

        // Filter letters by this task's ID
        let task_letters: Vec<&LetterRow> = app
            .data
            .letters
            .iter()
            .filter(|l| l.task_id == task.id)
            .collect();

        if task_letters.is_empty() {
            lines.push(Line::from("No letters for this task yet."));
        }

        for (idx, letter) in task_letters.iter().take(20).enumerate() {
            let to = letter.to_persona.as_deref().unwrap_or("broadcast");
            let header = format!(
                "[{}] {} → {}  ({})",
                idx + 1,
                letter.from_persona,
                to,
                letter.mood_at_send
            );
            lines.push(Line::from(vec![
                Span::styled(header, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            ]));
            let content = &letter.content;
            let max_len = inner.width as usize;
            for chunk in content.chars().collect::<Vec<_>>().chunks(max_len.saturating_sub(2)) {
                let s: String = chunk.iter().collect();
                lines.push(Line::from(vec![Span::raw(format!("  {}", s))]));
            }
            lines.push(Line::from(""));
        }
    } else {
        lines.push(Line::from("Select a task to see its letters."));
    }

    let total_lines = lines.len();
    let visible_lines = right_col[1].height as usize;
    let max_scroll = total_lines.saturating_sub(visible_lines.saturating_sub(2));
    app.letters_scroll = app.letters_scroll.min(max_scroll);

    let runner_para = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title("Mail / Runner Status"))
        .scroll((app.letters_scroll as u16, 0));

    frame.render_widget(runner_para, right_col[1]);

    // Scrollbar for letters
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .symbols(scrollbar::VERTICAL)
        .begin_symbol(Some("^"))
        .end_symbol(Some("v"));
    let mut scrollbar_state = ScrollbarState::new(total_lines)
        .position(app.letters_scroll)
        .viewport_content_length(visible_lines);
    frame.render_stateful_widget(
        scrollbar,
        right_col[1].inner(Margin {
            horizontal: 0,
            vertical: 1,
        }),
        &mut scrollbar_state,
    );

    // ── Status bar ──
    let status = format!(
        " q/esc quit  |  ↑/↓ or j/k tasks  |  ←/→ or h/l agents  |  pgup/pgdown scroll mail  |  r refresh  |  last refresh: {:?} ago ",
        app.last_refresh.elapsed()
    );
    let status_para = Paragraph::new(status)
        .style(Style::default().fg(Color::White).bg(Color::Black));
    frame.render_widget(status_para, main_layout[2]);
}

/// Convenience entry point used by `main.rs`.
/// Uses default paths relative to the current working directory.
pub fn run() -> Result<()> {
    let tasks_db = "openclaw-swarm.db";
    let knowledge_db = "scripts/swarm_knowledge.db";
    run_dashboard(tasks_db, knowledge_db)
}

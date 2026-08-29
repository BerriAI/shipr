use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Wrap};
use shipr_smart_routing::{resolve_routing_policy, select_model};
use std::io::{self, Stdout};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

const BLUE: Color = Color::Rgb(66, 153, 225);
const BLUE_DIM: Color = Color::Rgb(96, 165, 250);
const TEXT: Color = Color::Rgb(226, 232, 240);
const MUTED: Color = Color::Rgb(135, 145, 160);
const SURFACE: Color = Color::Rgb(49, 52, 59);
const BORDER: Color = Color::Rgb(72, 78, 90);

#[derive(Debug)]
enum FeedItem {
    Banner,
    User(String),
    Assistant(String),
    Meta(String),
}

#[derive(Debug, Clone, Copy)]
enum ActivityStage {
    Routing,
    Inspecting,
    Planning,
    Responding,
}

impl ActivityStage {
    fn description(self) -> &'static str {
        match self {
            Self::Routing => "Choosing the cheapest capable model",
            Self::Inspecting => "Reading context",
            Self::Planning => "Planning the response",
            Self::Responding => "Writing the response",
        }
    }
}

#[derive(Debug)]
enum WorkerEvent {
    Progress { task_id: u64, stage: ActivityStage },
    ResponseStarted { task_id: u64 },
    ResponseDelta { task_id: u64, delta: char },
    Done { task_id: u64, recap: String },
}

#[derive(Debug)]
struct App {
    input: String,
    feed: Vec<FeedItem>,
    processing: bool,
    activity: Option<ActivityStage>,
    started_processing: Option<Instant>,
    active_task_id: Option<u64>,
    next_task_id: u64,
    should_quit: bool,
    base_url: String,
}

impl App {
    fn new(base_url: String) -> Self {
        Self {
            input: String::new(),
            feed: vec![
                FeedItem::Banner,
                FeedItem::Meta("Type a coding task below. /help for commands.".to_string()),
            ],
            processing: false,
            activity: None,
            started_processing: None,
            active_task_id: None,
            next_task_id: 1,
            should_quit: false,
            base_url,
        }
    }

    fn submit(&mut self, sender: &Sender<WorkerEvent>) {
        let prompt = self.input.trim().to_string();
        if prompt.is_empty() || self.processing {
            return;
        }
        self.input.clear();

        if prompt.starts_with('/') {
            self.handle_command(&prompt);
            return;
        }

        let task_id = self.next_task_id;
        self.next_task_id += 1;
        self.active_task_id = Some(task_id);
        self.processing = true;
        self.activity = Some(ActivityStage::Routing);
        self.started_processing = Some(Instant::now());
        self.feed.push(FeedItem::User(prompt.clone()));

        spawn_task(task_id, prompt, self.base_url.clone(), sender.clone());
    }

    fn handle_command(&mut self, command: &str) {
        match command {
            "/exit" | "/quit" => self.should_quit = true,
            "/clear" => self.feed.clear(),
            "/help" => self.feed.push(FeedItem::Assistant(
                "/clear  clear conversation\n/help   show commands\n/exit   leave Shiprr"
                    .to_string(),
            )),
            _ => self
                .feed
                .push(FeedItem::Meta("Unknown command. Use /help.".to_string())),
        }
    }

    fn handle_worker_event(&mut self, event: WorkerEvent) {
        match event {
            WorkerEvent::Progress { task_id, stage } if self.active_task_id == Some(task_id) => {
                self.activity = Some(stage);
            }
            WorkerEvent::ResponseStarted { task_id } if self.active_task_id == Some(task_id) => {
                self.feed.push(FeedItem::Assistant(String::new()));
            }
            WorkerEvent::ResponseDelta { task_id, delta }
                if self.active_task_id == Some(task_id) =>
            {
                if let Some(FeedItem::Assistant(answer)) = self.feed.last_mut() {
                    answer.push(delta);
                }
            }
            WorkerEvent::Done { task_id, recap } if self.active_task_id == Some(task_id) => {
                self.feed.push(FeedItem::Meta(recap));
                self.processing = false;
                self.activity = None;
                self.started_processing = None;
                self.active_task_id = None;
            }
            _ => {}
        }
    }

    fn cancel(&mut self) {
        if self.processing {
            self.processing = false;
            self.activity = None;
            self.started_processing = None;
            self.active_task_id = None;
            self.feed
                .push(FeedItem::Meta("Task cancelled.".to_string()));
        }
    }
}

pub fn run(base_url: String) -> Result<()> {
    enable_raw_mode().context("failed to enable raw terminal mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("failed to enter terminal screen")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("failed to create terminal UI")?;

    let result = run_event_loop(&mut terminal, base_url);

    disable_raw_mode().context("failed to disable raw terminal mode")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("failed to leave terminal screen")?;
    terminal.show_cursor().context("failed to restore cursor")?;
    result
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    base_url: String,
) -> Result<()> {
    let (sender, receiver) = mpsc::channel();
    let mut app = App::new(base_url);

    while !app.should_quit {
        drain_worker_events(&receiver, &mut app);
        terminal.draw(|frame| draw(frame, &app))?;

        if event::poll(Duration::from_millis(80))?
            && let Event::Key(key) = event::read()?
        {
            handle_key(key, &mut app, &sender);
        }
    }

    Ok(())
}

fn drain_worker_events(receiver: &Receiver<WorkerEvent>, app: &mut App) {
    while let Ok(event) = receiver.try_recv() {
        app.handle_worker_event(event);
    }
}

fn handle_key(key: KeyEvent, app: &mut App, sender: &Sender<WorkerEvent>) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    match key.code {
        KeyCode::Esc => app.cancel(),
        KeyCode::Enter => app.submit(sender),
        KeyCode::Backspace => {
            app.input.pop();
        }
        KeyCode::Char(character) if !app.processing => app.input.push(character),
        _ => {}
    }
}

fn draw(frame: &mut ratatui::Frame<'_>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(if app.processing { 2 } else { 1 }),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(frame.area());

    draw_header(frame, chunks[0]);
    draw_feed(frame, chunks[1], app);
    draw_processing(frame, chunks[2], app);
    draw_composer(frame, chunks[3], app);
    draw_footer(frame, chunks[4], app);
}

fn draw_header(frame: &mut ratatui::Frame<'_>, area: Rect) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" ▣▣▣ ", Style::default().fg(BLUE_DIM)),
        Span::styled(
            " shiprr ",
            Style::default().fg(Color::Black).bg(BLUE).bold(),
        ),
        Span::styled("  minimal agentic coding CLI", Style::default().fg(MUTED)),
    ]))
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(BORDER)),
    );
    frame.render_widget(header, area);
}

fn draw_feed(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let lines = feed_lines(&app.feed);
    let visible_height = area.height.saturating_sub(1) as usize;
    let scroll = lines.len().saturating_sub(visible_height) as u16;
    let feed = Paragraph::new(lines)
        .style(Style::default().fg(TEXT))
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0))
        .block(Block::default().padding(Padding::new(2, 2, 1, 0)));
    frame.render_widget(feed, area);
}

fn feed_lines(items: &[FeedItem]) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for item in items {
        match item {
            FeedItem::Banner => {
                for line in [
                    "                 ┌──────┬──────┬──────┐",
                    "                 │  ▣▣  │  ▣▣  │  ▣▣  │",
                    "            ┌────┴──────┴──────┴──────┴────┐",
                    "        ____│          S H I P R R          │____",
                    "     __/____└──────────────────────────────┘____\\__",
                    "     ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~",
                    "           ship code. route smart. pay less.",
                    "",
                ] {
                    lines.push(Line::from(Span::styled(
                        line,
                        Style::default().fg(BLUE_DIM).bold(),
                    )));
                }
            }
            FeedItem::User(text) => {
                lines.push(Line::from(vec![
                    Span::styled("› ", Style::default().fg(BLUE).bold()),
                    Span::styled(text.clone(), Style::default().fg(TEXT).bold()),
                ]));
                lines.push(Line::default());
            }
            FeedItem::Assistant(text) => {
                lines.push(Line::default());
                for (index, part) in text.lines().enumerate() {
                    lines.push(Line::from(vec![
                        Span::styled(
                            if index == 0 { "● " } else { "  " },
                            Style::default().fg(Color::White),
                        ),
                        Span::styled(part.to_string(), Style::default().fg(TEXT)),
                    ]));
                }
                lines.push(Line::default());
            }
            FeedItem::Meta(text) => lines.push(Line::from(Span::styled(
                format!("✱ {text}"),
                Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
            ))),
        }
    }
    lines
}

fn draw_processing(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    if !app.processing {
        return;
    }

    let frame_index = app
        .started_processing
        .map(|started| (started.elapsed().as_millis() / 120) as usize)
        .unwrap_or_default();
    let spinner = ["✦", "✧", "◆", "◇"][frame_index % 4];
    let activity = app
        .activity
        .map(ActivityStage::description)
        .unwrap_or("Working");
    let processing = Paragraph::new(Line::from(vec![
        Span::styled(format!(" {spinner} "), Style::default().fg(BLUE).bold()),
        Span::styled("Processing…", Style::default().fg(BLUE_DIM).bold()),
        Span::styled(format!("  {activity}"), Style::default().fg(TEXT)),
        Span::styled("   esc to cancel", Style::default().fg(MUTED)),
    ]));
    frame.render_widget(processing, area);
}

fn draw_composer(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let border_style = if app.processing {
        Style::default().fg(BORDER)
    } else {
        Style::default().fg(BLUE)
    };
    let composer = Paragraph::new(Line::from(vec![
        Span::styled("› ", Style::default().fg(BLUE).bold()),
        Span::styled(app.input.clone(), Style::default().fg(TEXT)),
    ]))
    .style(Style::default().bg(SURFACE))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(" Ask Shiprr ", Style::default().fg(MUTED))),
    );
    frame.render_widget(composer, area);

    if !app.processing {
        let cursor_x = area
            .x
            .saturating_add(3)
            .saturating_add(app.input.len() as u16);
        let cursor_x = cursor_x.min(area.right().saturating_sub(2));
        frame.set_cursor_position(Position::new(cursor_x, area.y + 1));
    }
}

fn draw_footer(frame: &mut ratatui::Frame<'_>, area: Rect, app: &App) {
    let state = if app.processing { "working" } else { "ready" };
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" enter", Style::default().fg(BLUE_DIM)),
        Span::styled(" send  ", Style::default().fg(MUTED)),
        Span::styled("esc", Style::default().fg(BLUE_DIM)),
        Span::styled(" cancel  ", Style::default().fg(MUTED)),
        Span::styled("ctrl+c", Style::default().fg(BLUE_DIM)),
        Span::styled(" exit", Style::default().fg(MUTED)),
        Span::styled(
            format!("                                      {state}"),
            Style::default().fg(MUTED),
        ),
    ]));
    frame.render_widget(footer, area);
}

fn spawn_task(task_id: u64, prompt: String, base_url: String, sender: Sender<WorkerEvent>) {
    thread::spawn(move || {
        let routing = resolve_routing_policy(&prompt, None, None);
        let model = select_model(&routing.policy);
        let stages = [
            ActivityStage::Routing,
            ActivityStage::Inspecting,
            ActivityStage::Planning,
        ];

        for stage in stages {
            thread::sleep(Duration::from_millis(350));
            if sender
                .send(WorkerEvent::Progress { task_id, stage })
                .is_err()
            {
                return;
            }
        }

        let answer = answer_for(&prompt, model.name);
        if sender
            .send(WorkerEvent::Progress {
                task_id,
                stage: ActivityStage::Responding,
            })
            .is_err()
            || sender
                .send(WorkerEvent::ResponseStarted { task_id })
                .is_err()
        {
            return;
        }

        for delta in answer.chars() {
            if sender
                .send(WorkerEvent::ResponseDelta { task_id, delta })
                .is_err()
            {
                return;
            }
            thread::sleep(Duration::from_millis(12));
        }

        let recap = format!(
            "recap: routed via {} on {} · estimated tier {}",
            model.name, base_url, model.estimated_cost
        );
        let _ = sender.send(WorkerEvent::Done { task_id, recap });
    });
}

fn answer_for(prompt: &str, model: &str) -> String {
    let normalized = prompt.to_lowercase();
    if normalized.contains("who are you") || normalized.contains("your name") {
        return "I'm Shiprr, a minimal agentic coding CLI built on LiteLLM.\n\nI route each task to the cheapest capable model, then run a focused plan → work → verify loop."
            .to_string();
    }

    format!(
        "I processed the task with {model} and prepared the execution path.\n\nThe interactive work surface is active; the next implementation step is wiring these work events to real file, shell, and LiteLLM streaming tools."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submit_starts_task_and_clears_composer() {
        let (sender, _receiver) = mpsc::channel();
        let mut app = App::new("http://localhost:4000".to_string());
        app.input = "fix the tests".to_string();

        app.submit(&sender);

        assert!(app.processing);
        assert!(app.input.is_empty());
        assert_eq!(app.active_task_id, Some(1));
    }

    #[test]
    fn slash_exit_closes_the_app() {
        let mut app = App::new("http://localhost:4000".to_string());
        app.input = "/exit".to_string();
        let (sender, _receiver) = mpsc::channel();

        app.submit(&sender);

        assert!(app.should_quit);
    }

    #[test]
    fn response_deltas_stream_into_active_answer() {
        let mut app = App::new("http://localhost:4000".to_string());
        app.active_task_id = Some(7);

        app.handle_worker_event(WorkerEvent::ResponseStarted { task_id: 7 });
        app.handle_worker_event(WorkerEvent::ResponseDelta {
            task_id: 7,
            delta: 'S',
        });

        assert!(matches!(app.feed.last(), Some(FeedItem::Assistant(answer)) if answer == "S"));
    }
}

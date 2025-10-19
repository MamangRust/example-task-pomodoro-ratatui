use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Terminal,
};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    time::{Duration, Instant},
};

enum InputMode {
    Task,
    Language,
    NoTyping,
}

#[derive(Debug)]
enum PomodoroState {
    Idle,
    Work,
    Break,
}

const WORK_DURATION: Duration = Duration::from_secs(25 * 60);
const BREAK_DURATION: Duration = Duration::from_secs(5 * 60);
const MESSAGE_VISIBLE_FOR: Duration = Duration::from_secs(4);

struct Task {
    name: String,
    language: String,
    pomodoro_state: PomodoroState,
    pomodoro_start: Option<Instant>,
    completed_pomodoros: u32,
}

struct App {
    todos: Vec<Task>,
    input: String,
    language_input: String,
    selected_index: usize,
    input_mode: InputMode,
    cursor_position: usize,
    status_message: Option<(String, Instant)>,
}

impl App {
    fn new() -> Self {
        Self {
            todos: load_todos(),
            input: String::new(),
            language_input: String::new(),
            selected_index: 0,
            input_mode: InputMode::NoTyping,
            cursor_position: 0,
            status_message: None,
        }
    }

    fn start_pomodoro(&mut self) {
        if self.todos.is_empty() {
            return;
        }
        let task = &mut self.todos[self.selected_index];
        task.pomodoro_state = PomodoroState::Work;
        task.pomodoro_start = Some(Instant::now());
        self.status_message = Some((
            format!("Started focus on '{}'. Stay sharp!", task.name),
            Instant::now(),
        ));
    }

    fn update_pomodoro(&mut self) {
        if self.todos.is_empty() {
            return;
        }

        let task = &mut self.todos[self.selected_index];

        if let Some(start) = task.pomodoro_start {
            let elapsed = start.elapsed();

            match task.pomodoro_state {
                PomodoroState::Work if elapsed >= WORK_DURATION => {
                    task.pomodoro_state = PomodoroState::Break;
                    task.pomodoro_start = Some(Instant::now());
                    self.status_message = Some((
                        format!("Work session done! Take a break, {}.", task.name),
                        Instant::now(),
                    ));
                    task.completed_pomodoros += 1;
                }
                PomodoroState::Break if elapsed >= BREAK_DURATION => {
                    task.pomodoro_state = PomodoroState::Idle;
                    task.pomodoro_start = None;
                    self.status_message = Some((
                        "Break finished. Ready for another round?".to_string(),
                        Instant::now(),
                    ));
                }
                _ => {}
            }
        }
    }

    fn status_message(&mut self) -> Option<String> {
        if let Some((message, timestamp)) = &self.status_message {
            if timestamp.elapsed() < MESSAGE_VISIBLE_FOR {
                return Some(message.clone());
            }
        }
        self.status_message = None;
        None
    }

    fn pomodoro_overview(&self) -> (String, f64, Color) {
        if self.todos.is_empty() {
            return ("No tasks available".to_string(), 0.0, Color::DarkGray);
        }

        let task = &self.todos[self.selected_index];

        if let Some(start) = task.pomodoro_start {
            let elapsed = start.elapsed();
            let (phase, duration, color) = match task.pomodoro_state {
                PomodoroState::Work => ("Focus", WORK_DURATION, Color::LightGreen),
                PomodoroState::Break => ("Break", BREAK_DURATION, Color::LightBlue),
                PomodoroState::Idle => {
                    return (
                        "Pomodoro paused. Press 'p' to resume.".to_string(),
                        0.0,
                        Color::Gray,
                    )
                }
            };

            let remaining = duration
                .checked_sub(elapsed)
                .unwrap_or_else(|| Duration::from_secs(0));
            let progress = (elapsed.as_secs_f64() / duration.as_secs_f64()).min(1.0);

            (
                format!(
                    "{} — {:02}:{:02} left",
                    phase,
                    remaining.as_secs() / 60,
                    remaining.as_secs() % 60
                ),
                progress,
                color,
            )
        } else {
            (
                "Press 'p' to start the pomodoro for this task.".to_string(),
                0.0,
                Color::Gray,
            )
        }
    }

    fn handle_input(&mut self, c: char) {
        match self.input_mode {
            InputMode::Task => {
                self.input.push(c);
                self.cursor_position += 1;
            }
            InputMode::Language => {
                self.language_input.push(c);
                self.cursor_position += 1;
            }
            InputMode::NoTyping => {}
        }
    }

    fn handle_backspace(&mut self) {
        match self.input_mode {
            InputMode::Task => {
                if !self.input.is_empty() {
                    self.input.pop();
                    self.cursor_position = self.cursor_position.saturating_sub(1);
                }
            }
            InputMode::Language => {
                if !self.language_input.is_empty() {
                    self.language_input.pop();
                    self.cursor_position = self.cursor_position.saturating_sub(1);
                }
            }
            InputMode::NoTyping => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    loop {
        terminal.draw(|f| {
            let outer = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(5),
                    Constraint::Length(7),
                ])
                .split(f.area());

            let main_sections = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(outer[1]);

            let pomodoro_sections = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(3),
                ])
                .split(main_sections[1]);

            let todo_items: Vec<ListItem> = app
                .todos
                .iter()
                .enumerate()
                .map(|(i, task)| {
                    let (state_label, color) = match task.pomodoro_state {
                        PomodoroState::Idle => ("Idle", Color::Gray),
                        PomodoroState::Work => ("Focus", Color::LightGreen),
                        PomodoroState::Break => ("Break", Color::LightBlue),
                    };

                    let primary = format!("{} · {}", task.name, task.language);
                    let secondary = format!(
                        "Status: {} | Completed: {}",
                        state_label, task.completed_pomodoros
                    );

                    let lines = vec![
                        Line::from(primary),
                        Line::from(Span::styled(secondary, Style::default().fg(color))),
                    ];

                    let mut list_item = ListItem::new(lines);
                    if i == app.selected_index {
                        list_item = list_item.style(
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        );
                    }
                    list_item
                })
                .collect();

            let list = List::new(todo_items)
                .block(Block::default().borders(Borders::ALL).title("To-Do List"));

            let input_title = match app.input_mode {
                InputMode::Task => "New Task (Task Input Mode)",
                InputMode::Language => "New Task (Language Input Mode)",
                InputMode::NoTyping => "New Task (Press 'i' to add)",
            };

            let input_lines = vec![
                Line::from(vec![
                    Span::styled("Task:", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(format!(" {}", app.input)),
                ]),
                Line::from(vec![
                    Span::styled("Language:", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(format!(" {}", app.language_input)),
                ]),
                Line::from("Enter to confirm, ESC to cancel"),
            ];

            let input_box = Paragraph::new(input_lines)
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL).title(input_title));

            let (status_text, progress, color) = app.pomodoro_overview();
            let gauge = Gauge::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Pomodoro Progress"),
                )
                .gauge_style(
                    Style::default()
                        .fg(color)
                        .bg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                )
                .label(status_text)
                .ratio(progress);

            let mut info_lines = vec![Line::from(vec![
                Span::styled(
                    "Controls:",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  i=add task  ↑/↓=navigate  p=start timer  del=remove  q=quit"),
            ])];

            if let Some(message) = app.status_message() {
                info_lines.push(Line::from(Span::styled(
                    message,
                    Style::default()
                        .fg(Color::LightCyan)
                        .add_modifier(Modifier::ITALIC),
                )));
            }

            let info_box = Paragraph::new(info_lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Session Overview"),
            );

            let summary_lines = vec![
                Line::from(vec![
                    Span::styled(
                        "Selected Task:",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" (press p to begin)"),
                ]),
                if app.todos.is_empty() {
                    Line::from("No task selected")
                } else {
                    let task = &app.todos[app.selected_index];
                    Line::from(format!(
                        "{} | {} | Completed focus sessions: {}",
                        task.name, task.language, task.completed_pomodoros
                    ))
                },
            ];

            let summary_box = Paragraph::new(summary_lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Task Snapshot"),
            );

            let header = Paragraph::new(vec![Line::from(vec![
                Span::styled(
                    "⚡ Pomodoro Control Center",
                    Style::default()
                        .fg(Color::LightMagenta)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" — Stay focused and track your progress"),
            ])])
            .block(
                Block::default()
                    .style(Style::default().bg(Color::Black))
                    .title(Span::styled(
                        " Focus Mode ",
                        Style::default()
                            .fg(Color::LightMagenta)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .borders(Borders::ALL),
            );

            f.render_widget(header, outer[0]);
            f.render_widget(list, main_sections[0]);
            f.render_widget(gauge, pomodoro_sections[0]);
            f.render_widget(info_box, pomodoro_sections[1]);
            f.render_widget(summary_box, pomodoro_sections[2]);
            f.render_widget(input_box, outer[2]);
        })?;

        app.update_pomodoro();

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Esc => {
                        app.input_mode = InputMode::NoTyping;
                        app.input.clear();
                        app.language_input.clear();
                        app.status_message =
                            Some(("Creation cancelled.".to_string(), Instant::now()));
                    }
                    KeyCode::Char('p') => {
                        app.start_pomodoro();
                    }
                    KeyCode::Char('i') => {
                        app.input_mode = InputMode::Task;
                        app.cursor_position = 0;
                    }
                    KeyCode::Char(c) => app.handle_input(c),
                    KeyCode::Backspace => app.handle_backspace(),
                    KeyCode::Enter => match app.input_mode {
                        InputMode::Task => {
                            if !app.input.trim().is_empty() {
                                app.input_mode = InputMode::Language;
                                app.cursor_position = 0;
                            }
                        }
                        InputMode::Language => {
                            if !app.language_input.trim().is_empty() {
                                app.todos.push(Task {
                                    name: app.input.trim().to_string(),
                                    language: app.language_input.trim().to_string(),
                                    pomodoro_state: PomodoroState::Idle,
                                    pomodoro_start: None,
                                    completed_pomodoros: 0,
                                });
                                save_todos(&app.todos);
                                app.input.clear();
                                app.language_input.clear();
                                app.input_mode = InputMode::NoTyping;
                                app.cursor_position = 0;
                                app.status_message = Some((
                                    "New task added. Ready to focus!".to_string(),
                                    Instant::now(),
                                ));
                            }
                        }
                        InputMode::NoTyping => {}
                    },
                    KeyCode::Delete if !app.todos.is_empty() => {
                        let removed = app.todos.remove(app.selected_index);
                        save_todos(&app.todos);
                        if app.selected_index >= app.todos.len() && app.selected_index > 0 {
                            app.selected_index -= 1;
                        }
                        app.status_message =
                            Some((format!("Removed '{}'.", removed.name), Instant::now()));
                    }
                    KeyCode::Up if app.selected_index > 0 => {
                        app.selected_index -= 1;
                    }
                    KeyCode::Down if app.selected_index < app.todos.len().saturating_sub(1) => {
                        app.selected_index += 1;
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn load_todos() -> Vec<Task> {
    match fs::read_to_string("todo_list.txt") {
        Ok(content) => content
            .lines()
            .map(|s| {
                let parts: Vec<&str> = s.split(" | ").collect();
                let completed = parts
                    .get(2)
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or(0);
                Task {
                    name: parts[0].to_string(),
                    language: parts.get(1).unwrap_or(&"Unknown").to_string(),
                    pomodoro_state: PomodoroState::Idle,
                    pomodoro_start: None,
                    completed_pomodoros: completed,
                }
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn save_todos(todos: &Vec<Task>) {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("todo_list.txt")
        .unwrap();
    for task in todos {
        writeln!(
            file,
            "{} | {} | {}",
            task.name, task.language, task.completed_pomodoros
        )
        .unwrap();
    }
}

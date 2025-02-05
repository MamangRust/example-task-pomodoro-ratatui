use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
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

struct Task {
    name: String,
    language: String,
    pomodoro_state: PomodoroState,
    pomodoro_start: Option<Instant>,
}

struct App {
    todos: Vec<Task>,
    input: String,
    language_input: String,
    selected_index: usize,
    input_mode: InputMode,
    cursor_position: usize,
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
        }
    }

    fn start_pomodoro(&mut self) {
        if self.todos.is_empty() {
            return;
        }
        let task = &mut self.todos[self.selected_index];
        task.pomodoro_state = PomodoroState::Work;
        task.pomodoro_start = Some(Instant::now());
    }

    fn update_pomodoro(&mut self) {
        if self.todos.is_empty() {
            return;
        }

        let task = &mut self.todos[self.selected_index];

        if let Some(start) = task.pomodoro_start {
            let elapsed = start.elapsed();

            match task.pomodoro_state {
                PomodoroState::Work if elapsed >= Duration::from_secs(1500) => {
                    println!("Switching to Break Mode");
                    task.pomodoro_state = PomodoroState::Break;
                    task.pomodoro_start = Some(Instant::now());
                }
                PomodoroState::Break if elapsed >= Duration::from_secs(300) => {
                    println!("Pomodoro Finished");
                    task.pomodoro_state = PomodoroState::Idle;
                    task.pomodoro_start = None;
                }
                _ => {}
            }
        }
    }

    fn get_pomodoro_time(&self) -> String {
        if self.todos.is_empty() {
            return "Pomodoro: Not running".to_string();
        }

        let task = &self.todos[self.selected_index];

        if let Some(start) = task.pomodoro_start {
            let elapsed = start.elapsed();
            let remaining = match task.pomodoro_state {
                PomodoroState::Work => 1500 - elapsed.as_secs(),
                PomodoroState::Break => 300 - elapsed.as_secs(),
                PomodoroState::Idle => return "Pomodoro: Not running".to_string(),
            };

            format!(
                "Pomodoro: {} - {}:{:02}",
                match task.pomodoro_state {
                    PomodoroState::Work => "Work",
                    PomodoroState::Break => "Break",
                    PomodoroState::Idle => "Idle",
                },
                remaining / 60,
                remaining % 60
            )
        } else {
            "Pomodoro: Not running".to_string()
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
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(70),
                    Constraint::Percentage(15),
                    Constraint::Percentage(15),
                ])
                .split(f.area());

            let todo_items: Vec<ListItem> = app
                .todos
                .iter()
                .enumerate()
                .map(|(i, task)| {
                    let style = if i == app.selected_index {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    };
                    ListItem::new(format!("{} [{}]", task.name, task.language)).style(style)
                })
                .collect();

            let list = List::new(todo_items)
                .block(Block::default().borders(Borders::ALL).title("To-Do List"));

            let input_title = match app.input_mode {
                InputMode::Task => "New Task (Task Input Mode)",
                InputMode::Language => "New Task (Language Input Mode)",
                InputMode::NoTyping => "New Task (Press 'i' to add)",
            };

            let input_text = format!("Task: {}\nLang: {}", app.input, app.language_input);

            let input_box = Paragraph::new(input_text)
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL).title(input_title));

            let pomodoro_box = Paragraph::new(app.get_pomodoro_time()).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Pomodoro Timer"),
            );

            f.render_widget(list, chunks[0]);
            f.render_widget(input_box, chunks[1]);
            f.render_widget(pomodoro_box, chunks[2]);
        })?;

        app.update_pomodoro();

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
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
                                });
                                save_todos(&app.todos);
                                app.input.clear();
                                app.language_input.clear();
                                app.input_mode = InputMode::NoTyping;
                                app.cursor_position = 0;
                            }
                        }
                        InputMode::NoTyping => {}
                    },
                    KeyCode::Delete if !app.todos.is_empty() => {
                        app.todos.remove(app.selected_index);
                        save_todos(&app.todos);
                        if app.selected_index >= app.todos.len() && app.selected_index > 0 {
                            app.selected_index -= 1;
                        }
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
                Task {
                    name: parts[0].to_string(),
                    language: parts.get(1).unwrap_or(&"Unknown").to_string(),
                    pomodoro_state: PomodoroState::Idle,
                    pomodoro_start: None,
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
        writeln!(file, "{} | {}", task.name, task.language).unwrap();
    }
}

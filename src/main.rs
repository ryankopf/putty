use std::fs;
use std::io::{self, stdout};
use std::path::PathBuf;
use std::time::Duration;
use std::thread::sleep;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute, terminal,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};

#[derive(Debug, Clone)]
struct HostEntry {
    name: String,
    hostname: Option<String>,
    user: Option<String>,
}

impl HostEntry {
    fn parse_ssh_config(file: &str) -> Vec<HostEntry> {
        let mut hosts = Vec::new();
        let mut current: Option<HostEntry> = None;

        for line in file.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("Host ") {
                if let Some(entry) = current.take() {
                    hosts.push(entry);
                }
                let name = trimmed.strip_prefix("Host").unwrap().trim().to_string();
                current = Some(HostEntry {
                    name,
                    hostname: None,
                    user: None,
                });
            } else if let Some(entry) = current.as_mut() {
                if let Some(rest) = trimmed.strip_prefix("HostName") {
                    entry.hostname = Some(rest.trim().to_string());
                } else if let Some(rest) = trimmed.strip_prefix("User") {
                    entry.user = Some(rest.trim().to_string());
                }
            }
        }

        if let Some(entry) = current {
            hosts.push(entry);
        }

        hosts
    }
}

struct AppState {
    hosts: Vec<HostEntry>,
    selected: usize,
    last_key: Option<KeyCode>,
    last_key_time: Option<std::time::Instant>,
}

impl AppState {
    fn new(hosts: Vec<HostEntry>) -> Self {
        Self {
            hosts,
            selected: 0,
            last_key: None,
            last_key_time: None,
        }
    }
    fn update_selection(&mut self) {
        // No-op, kept for compatibility if needed
    }
}

fn load_config_file() -> io::Result<Vec<HostEntry>> {
    let path = ssh_config_path();
    let contents = fs::read_to_string(path)?;
    Ok(HostEntry::parse_ssh_config(&contents))
}

fn ssh_config_path() -> PathBuf {
    let home = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(format!("{}\\.ssh\\config", home))
}


fn draw_ui(
    f: &mut ratatui::Frame,
    app: &AppState,
    config_path_str: &str,
) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(area);

    let items: Vec<ListItem> = if app.hosts.is_empty() {
        vec![ListItem::new("No hosts found.")]
    } else {
        app.hosts
            .iter()
            .enumerate()
            .map(|(i, h)| {
                let mut label = h.name.clone();
                if let Some(ip) = &h.hostname {
                    label.push_str(&format!(" ({})", ip));
                }
                let mut item = ListItem::new(Text::from(Line::from(Span::raw(label))));
                if i == app.selected {
                    item = item.style(Style::default().add_modifier(Modifier::REVERSED));
                }
                item
            })
            .collect()
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(format!("SSH Hosts ({})", config_path_str)))
        .highlight_symbol("→ ");

    f.render_widget(list, chunks[0]);

    let edit = Paragraph::new("Press [e] to edit a host, [q] to quit")
        .block(Block::default().borders(Borders::ALL).title("Controls"));
    f.render_widget(edit, chunks[1]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let hosts = load_config_file().unwrap_or_default();
    let config_path = ssh_config_path();
    let config_path_str = config_path.display().to_string();
    let mut app = AppState::new(hosts);

    loop {
        terminal.draw(|f| draw_ui(f, &app, &config_path_str))?;

        if let Event::Key(key) = event::read()? {
            let now = std::time::Instant::now();
            let allow = match (app.last_key, app.last_key_time) {
                (Some(prev), Some(t)) if prev == key.code && now.duration_since(t) < Duration::from_millis(200) => false,
                _ => true,
            };
            if allow {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('e') => {
                        if !app.hosts.is_empty() {
                            println!("Edit host: {}", app.hosts[app.selected].name);
                        }
                    }
                    KeyCode::Down => {
                        if !app.hosts.is_empty() {
                            app.selected = (app.selected + 1) % app.hosts.len();
                        }
                    }
                    KeyCode::Up => {
                        if !app.hosts.is_empty() {
                            if app.selected == 0 {
                                app.selected = app.hosts.len() - 1;
                            } else {
                                app.selected -= 1;
                            }
                        }
                    }
                    _ => {}
                }
                app.last_key = Some(key.code);
                app.last_key_time = Some(now);
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

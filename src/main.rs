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
    port: Option<String>,
    identity_file: Option<String>,
    proxy_jump: Option<String>,
    forward_agent: Option<String>,
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
                    port: None,
                    identity_file: None,
                    proxy_jump: None,
                    forward_agent: None,
                });
            } else if let Some(entry) = current.as_mut() {
                if let Some(rest) = trimmed.strip_prefix("HostName") {
                    entry.hostname = Some(rest.trim().to_string());
                } else if let Some(rest) = trimmed.strip_prefix("User") {
                    entry.user = Some(rest.trim().to_string());
                } else if let Some(rest) = trimmed.strip_prefix("Port") {
                    entry.port = Some(rest.trim().to_string());
                } else if let Some(rest) = trimmed.strip_prefix("IdentityFile") {
                    entry.identity_file = Some(rest.trim().to_string());
                } else if let Some(rest) = trimmed.strip_prefix("ProxyJump") {
                    entry.proxy_jump = Some(rest.trim().to_string());
                } else if let Some(rest) = trimmed.strip_prefix("ForwardAgent") {
                    entry.forward_agent = Some(rest.trim().to_string());
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
    edit_mode: Option<EditState>,
}

#[derive(Debug, Clone)]
struct EditState {
    host: HostEntry,
    field_index: usize,
    field_values: Vec<String>,
}

impl AppState {
    fn new(hosts: Vec<HostEntry>) -> Self {
        Self {
            hosts,
            selected: 0,
            last_key: None,
            last_key_time: None,
            edit_mode: None,
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

    if let Some(edit) = &app.edit_mode {
        // Edit mode UI
        let fields = [
            ("Host", edit.host.name.clone()),
            ("HostName", edit.host.hostname.clone().unwrap_or_default()),
            ("User", edit.host.user.clone().unwrap_or_default()),
            ("Port", edit.host.port.clone().unwrap_or_default()),
            ("IdentityFile", edit.host.identity_file.clone().unwrap_or_default()),
            ("ProxyJump", edit.host.proxy_jump.clone().unwrap_or_default()),
            ("ForwardAgent", edit.host.forward_agent.clone().unwrap_or_default()),
        ];
        let items: Vec<ListItem> = fields.iter().enumerate().map(|(i, (label, value))| {
            let mut line = format!("{}: {}", label, value);
            if i == edit.field_index {
                line.push_str(" <");
            }
            let mut item = ListItem::new(line);
            if i == edit.field_index {
                item = item.style(Style::default().add_modifier(Modifier::REVERSED));
            }
            item
        }).collect();
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Edit Host"))
            .highlight_symbol("→ ");
        f.render_widget(list, chunks[0]);
        let edit = Paragraph::new("[Enter] Save  [Esc] Cancel  [Tab/Up/Down] Move  Type to edit")
            .block(Block::default().borders(Borders::ALL).title("Editing"));
        f.render_widget(edit, chunks[1]);
    } else {
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
                if let Some(edit) = &mut app.edit_mode {
                    // Edit mode key handling
                    match key.code {
                        KeyCode::Esc => {
                            app.edit_mode = None;
                        }
                        KeyCode::Enter => {
                            // Save changes
                            if app.selected < app.hosts.len() {
                                app.hosts[app.selected] = edit.host.clone();
                            }
                            app.edit_mode = None;
                        }
                        KeyCode::Tab | KeyCode::Down => {
                            edit.field_index = (edit.field_index + 1) % 7;
                        }
                        KeyCode::Up => {
                            if edit.field_index == 0 {
                                edit.field_index = 6;
                            } else {
                                edit.field_index -= 1;
                            }
                        }
                        KeyCode::Backspace => {
                            let field = get_edit_field_mut(&mut edit.host, edit.field_index);
                            if let Some(val) = field {
                                val.pop();
                            }
                        }
                        KeyCode::Char(c) => {
                            let field = get_edit_field_mut(&mut edit.host, edit.field_index);
                            if let Some(val) = field {
                                val.push(c);
                            }
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('e') => {
                            if !app.hosts.is_empty() {
                                let host = app.hosts[app.selected].clone();
                                app.edit_mode = Some(EditState {
                                    host,
                                    field_index: 0,
                                    field_values: vec![], // unused for now
                                });
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

fn get_edit_field_mut<'a>(host: &'a mut HostEntry, idx: usize) -> Option<&'a mut String> {
    match idx {
        0 => Some(&mut host.name),
        1 => {
            if host.hostname.is_none() { host.hostname = Some(String::new()); }
            host.hostname.as_mut()
        },
        2 => {
            if host.user.is_none() { host.user = Some(String::new()); }
            host.user.as_mut()
        },
        3 => {
            if host.port.is_none() { host.port = Some(String::new()); }
            host.port.as_mut()
        },
        4 => {
            if host.identity_file.is_none() { host.identity_file = Some(String::new()); }
            host.identity_file.as_mut()
        },
        5 => {
            if host.proxy_jump.is_none() { host.proxy_jump = Some(String::new()); }
            host.proxy_jump.as_mut()
        },
        6 => {
            if host.forward_agent.is_none() { host.forward_agent = Some(String::new()); }
            host.forward_agent.as_mut()
        },
        _ => None,
    }
}

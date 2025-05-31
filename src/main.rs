use std::fs;
use std::io::{self, stdout};
use std::path::PathBuf;
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

fn load_config_file() -> io::Result<Vec<HostEntry>> {
    let path = ssh_config_path();
    let contents = fs::read_to_string(path)?;
    Ok(parse_ssh_config(&contents))
}

fn ssh_config_path() -> PathBuf {
    let home = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(format!("{}\\.ssh\\config", home))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let hosts = load_config_file().unwrap_or_default();
    let mut selected = 0;
    let config_path = ssh_config_path();
    let config_path_str = config_path.display().to_string();

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Min(3),
                    Constraint::Length(3),
                ])
                .split(size);

            let items: Vec<ListItem> = if hosts.is_empty() {
                vec![ListItem::new("No hosts found.")]
            } else {
                hosts
                    .iter()
                    .map(|h| {
                        let mut label = h.name.clone();
                        if let Some(ip) = &h.hostname {
                            label.push_str(&format!(" ({})", ip));
                        }
                        ListItem::new(Text::from(Line::from(Span::raw(label))))
                    })
                    .collect()
            };

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(format!("SSH Hosts ({})", config_path_str)))
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                .highlight_symbol("→ ");

            f.render_stateful_widget(list, chunks[0], &mut ratatui::widgets::ListState::default());

            let edit = Paragraph::new("Press [e] to edit a host, [q] to quit")
                .block(Block::default().borders(Borders::ALL).title("Controls"));
            f.render_widget(edit, chunks[1]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('e') => {
                    // TODO: Jump to edit view
                }
                _ => {}
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

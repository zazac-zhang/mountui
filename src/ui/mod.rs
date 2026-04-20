mod mount_list;
mod bookmark_list;
mod mount_point;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Tabs, Paragraph, Wrap},
};

use crate::app::{App, Mode, Tab};

/// Main render function - draws the entire UI.
pub fn render(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    let vertical = Layout::vertical([
        Constraint::Length(3), // tabs
        Constraint::Min(5),   // content
        Constraint::Length(1), // status bar
        Constraint::Length(1), // help bar
    ]);
    let [tabs_area, content_area, status_area, help_area] = vertical.areas(size);

    render_tabs(frame, tabs_area, app);
    render_content(frame, content_area, app);
    render_status_bar(frame, status_area, app);
    render_help_bar(frame, help_area, app);
}

fn render_tabs(frame: &mut Frame, area: Rect, app: &App) {
    let titles: Vec<Line> = Tab::ALL
        .iter()
        .map(|t| {
            let style = if *t == app.tab {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Line::from(Span::styled(t.title(), style))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" MountUI "),
        )
        .select(app.tab.index())
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    frame.render_widget(tabs, area);
}

fn render_content(frame: &mut Frame, area: Rect, app: &mut App) {
    match app.tab {
        Tab::Mounts => mount_list::render(frame, area, app),
        Tab::Bookmarks => bookmark_list::render(frame, area, app),
        Tab::MountPoints => mount_point::render(frame, area, app),
    }
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let style = match app.mode {
        Mode::Search => Style::default().fg(Color::Yellow),
        Mode::Form => Style::default().fg(Color::Magenta),
        Mode::Input => Style::default().fg(Color::Cyan),
        Mode::Normal => {
            if let Some(ref msg) = app.status {
                match msg.kind {
                    crate::app::StatusKind::Info => Style::default().fg(Color::White),
                    crate::app::StatusKind::Success => Style::default().fg(Color::Green),
                    crate::app::StatusKind::Error => Style::default().fg(Color::Red),
                }
            } else {
                Style::default().fg(Color::DarkGray)
            }
        }
    };

    let text = match app.mode {
        Mode::Search => format!("Search: {}", app.search_query),
        Mode::Form => "Form mode".to_string(),
        Mode::Input => format!("Create mount point: {}", app.input_buffer),
        Mode::Normal => {
            app.status
                .as_ref()
                .map(|s| s.text.clone())
                .unwrap_or_default()
        }
    };

    let paragraph = Paragraph::new(text).style(style);
    frame.render_widget(paragraph, area);
}

fn render_help_bar(frame: &mut Frame, area: Rect, app: &App) {
    let bindings = match app.mode {
        Mode::Normal => match app.tab {
            Tab::Mounts => {
                "[j/k] Navigate  [u]Unmount  [r]Refresh  [/]Search  [1-3] Tab  [q]Quit"
            }
            Tab::Bookmarks => {
                "[j/k] Navigate  [m]Mount  [d]Delete  [a]Add  [e]Edit  [/]Search  [1-3] Tab  [q]Quit"
            }
            Tab::MountPoints => {
                "[j/k] Navigate  [c]Create  [x]Remove  [/]Search  [1-3] Tab  [q]Quit"
            }
        },
        Mode::Search => "[Esc] Cancel  [Enter] Confirm  type to search",
        Mode::Form => "[Tab] Next field  [Esc] Cancel  [Enter] Confirm",
        Mode::Input => "[Esc] Cancel  [Enter] Confirm  type mount point path",
    };

    let paragraph = Paragraph::new(bindings)
        .style(Style::default().fg(Color::DarkGray))
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

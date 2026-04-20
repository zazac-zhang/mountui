use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Cell, Row, Table, TableState},
};

use crate::app::App;

const REMOTE_FS_TYPES: &[&str] = &[
    "nfs",
    "sshfs",
    "smbfs",
    "cifs",
    "fuse.sshfs",
    "fuse.osxfuse",
];

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let filtered = app.filtered_mounts();

    if filtered.is_empty() {
        let msg = if app.search_query.is_empty() {
            "No mounts found"
        } else {
            "No mounts matching search"
        };
        let paragraph = ratatui::widgets::Paragraph::new(msg)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(paragraph, area);
        return;
    }

    let header = Row::new(vec![
        Cell::from("Device").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Mount Point").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Type").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Options").style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .style(Style::default().bg(Color::DarkGray).fg(Color::White))
    .height(1);

    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let is_remote = REMOTE_FS_TYPES.contains(&entry.fs_type.as_str());
            let row_style = if i == app.cursor && app.tab == crate::app::Tab::Mounts {
                Style::default()
                    .bg(Color::Rgb(0, 90, 90))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if is_remote {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(truncate(&entry.device, 30)),
                Cell::from(truncate(&entry.mount_point.to_string_lossy(), 30)),
                Cell::from(truncate(&entry.fs_type, 10)),
                Cell::from(truncate(&entry.options, 40)),
            ])
            .style(row_style)
            .height(1)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            ratatui::layout::Constraint::Percentage(30),
            ratatui::layout::Constraint::Percentage(25),
            ratatui::layout::Constraint::Percentage(10),
            ratatui::layout::Constraint::Percentage(35),
        ],
    )
    .header(header)
    .row_highlight_style(Style::default().bg(Color::Rgb(0, 90, 90)));

    let mut state = TableState::default();
    state.select(Some(app.cursor));
    frame.render_stateful_widget(table, area, &mut state);
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{truncated}\u{2026}")
    }
}

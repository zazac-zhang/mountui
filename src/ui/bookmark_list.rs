use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
};

use crate::app::{App, FormField};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    if app.form.is_some() {
        render_form(frame, area, app);
        return;
    }

    let filtered = app.filtered_bookmarks();

    if filtered.is_empty() {
        let msg = if app.search_query.is_empty() {
            "No bookmarks. Press 'a' to add one."
        } else {
            "No bookmarks matching search"
        };
        let paragraph = Paragraph::new(msg)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(paragraph, area);
        return;
    }

    let header = Row::new(vec![
        Cell::from("Name").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Protocol").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Host").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Remote Path").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Mount Point").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Status").style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .style(Style::default().bg(Color::DarkGray).fg(Color::White))
    .height(1);

    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .map(|(i, bm)| {
            let mounted = app.is_bookmark_mounted(bm);
            let row_style = if i == app.cursor && app.tab == crate::app::Tab::Bookmarks {
                Style::default()
                    .bg(Color::Rgb(0, 90, 90))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if mounted {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            let status = if mounted { "mounted" } else { "\u{2014}" };

            Row::new(vec![
                Cell::from(truncate(&bm.name, 15)),
                Cell::from(bm.protocol.to_string()),
                Cell::from(truncate(&bm.host, 20)),
                Cell::from(truncate(&bm.remote_path, 25)),
                Cell::from(truncate(&bm.mount_point, 25)),
                Cell::from(status),
            ])
            .style(row_style)
            .height(1)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            ratatui::layout::Constraint::Percentage(15),
            ratatui::layout::Constraint::Percentage(8),
            ratatui::layout::Constraint::Percentage(20),
            ratatui::layout::Constraint::Percentage(22),
            ratatui::layout::Constraint::Percentage(25),
            ratatui::layout::Constraint::Percentage(10),
        ],
    )
    .header(header)
    .row_highlight_style(Style::default().bg(Color::Rgb(0, 90, 90)));

    let mut state = TableState::default();
    state.select(Some(app.cursor));
    frame.render_stateful_widget(table, area, &mut state);
}

fn render_form(frame: &mut Frame, area: Rect, app: &App) {
    let form = match &app.form {
        Some(f) => f,
        None => return,
    };

    let title = if form.editing {
        "Edit Bookmark"
    } else {
        "Add Bookmark"
    };

    let lines: Vec<Line> = FormField::LABELS
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let value = &form.fields[i];
            let is_active = i == form.cursor_field;

            let label_style = if is_active {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let value_style = if is_active {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Gray)
            };

            let cursor_char = if is_active { "\u{2588}" } else { "" };

            Line::from(vec![
                Span::styled(format!(" {label}: "), label_style),
                Span::styled(format!("{value}{cursor_char}"), value_style),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {title} ")),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{truncated}\u{2026}")
    }
}

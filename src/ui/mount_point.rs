use std::path::PathBuf;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Cell, Row, Table, TableState},
};

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let mount_points = collect_mount_points(app);

    if mount_points.is_empty() {
        let paragraph = ratatui::widgets::Paragraph::new("No mount points")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(paragraph, area);
        return;
    }

    let header = Row::new(vec![
        Cell::from("Mount Point").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Status").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Device").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Type").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Usage").style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .style(Style::default().bg(Color::DarkGray).fg(Color::White))
    .height(1);

    let rows: Vec<Row> = mount_points
        .iter()
        .enumerate()
        .map(|(i, mp)| {
            let mounted = app.mounts.iter().any(|m| m.mount_point == mp.path);
            let row_style = if i == app.cursor && app.tab == crate::app::Tab::MountPoints {
                Style::default().bg(Color::Rgb(0, 90, 90)).fg(Color::White).add_modifier(Modifier::BOLD)
            } else if mounted {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let status = if mounted { "mounted" } else { "\u{2014}" };

            let usage_text = if let Some(ref usage) = mp.disk_usage {
                format_usage(usage)
            } else {
                "\u{2014}".to_string()
            };

            Row::new(vec![
                Cell::from(truncate(&mp.path.to_string_lossy(), 35)),
                Cell::from(status),
                Cell::from(truncate(&mp.device, 25)),
                Cell::from(truncate(&mp.fs_type, 10)),
                Cell::from(usage_text),
            ])
            .style(row_style)
            .height(1)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            ratatui::layout::Constraint::Percentage(35),
            ratatui::layout::Constraint::Percentage(10),
            ratatui::layout::Constraint::Percentage(25),
            ratatui::layout::Constraint::Percentage(10),
            ratatui::layout::Constraint::Percentage(20),
        ],
    )
    .header(header)
    .row_highlight_style(Style::default().bg(Color::Rgb(0, 90, 90)));

    let mut state = TableState::default();
    state.select(Some(app.cursor));
    frame.render_stateful_widget(table, area, &mut state);
}

struct MountPointInfo {
    path: PathBuf,
    device: String,
    fs_type: String,
    disk_usage: Option<crate::mount::DiskUsage>,
}

fn collect_mount_points(app: &mut App) -> Vec<MountPointInfo> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();

    // Collect basic info from mounts first (immutable borrow)
    let mount_entries: Vec<(PathBuf, String, String)> = app
        .mounts
        .iter()
        .map(|m| (m.mount_point.clone(), m.device.clone(), m.fs_type.clone()))
        .collect();

    // Now query disk usage with mutable borrow
    for (path, device, fs_type) in mount_entries {
        if seen.insert(path.clone()) {
            let usage = app.get_disk_usage(&path);
            result.push(MountPointInfo {
                path,
                device,
                fs_type,
                disk_usage: usage,
            });
        }
    }

    // From bookmarks (unmounted) — only needs immutable access
    for b in &app.bookmarks {
        let mp = PathBuf::from(&b.mount_point);
        if seen.insert(mp.clone()) {
            result.push(MountPointInfo {
                path: mp,
                device: format!("{}:{}", b.host, b.remote_path),
                fs_type: b.protocol.to_string(),
                disk_usage: None,
            });
        }
    }

    // Apply search filter (path only, consistent with App::filtered_mount_point_paths)
    if !app.search_query.is_empty() {
        let q = app.search_query.to_lowercase();
        result.retain(|mp| {
            mp.path.to_string_lossy().to_lowercase().contains(&q)
        });
    }

    result
}

fn format_usage(usage: &crate::mount::DiskUsage) -> String {
    let pct = usage.usage_percent();
    format!(
        "{} / {} ({:.0}%)",
        format_bytes(usage.used),
        format_bytes(usage.total),
        pct,
    )
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    const TB: u64 = 1024 * GB;

    if bytes >= TB {
        format!("{:.1}T", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else {
        format!("{:.1}K", bytes as f64 / KB as f64)
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{truncated}\u{2026}")
    }
}

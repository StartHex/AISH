use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Row, Table},
    Frame,
};

use crate::app::App;

pub fn render_bands(f: &mut Frame, app: &App, area: Rect) {
    let rows: Vec<Row> = app
        .bands
        .iter()
        .map(|b| {
            let (status_icon, _) = match b.status.as_str() {
                "active" => ("●", Color::Green),
                "stopped" => ("○", Color::DarkGray),
                _ => ("◌", Color::Yellow),
            };

            Row::new(vec![
                format!("{} {}", status_icon, b.name),
                b.isolation.clone(),
                b.root.clone(),
                b.status.clone(),
            ])
            .style(Style::default().fg(Color::White))
        })
        .collect();

    let widths = [
        ratatui::layout::Constraint::Length(16),
        ratatui::layout::Constraint::Length(14),
        ratatui::layout::Constraint::Percentage(40),
        ratatui::layout::Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["Name", "Isolation", "Root", "Status"]).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(Block::default().borders(Borders::ALL).title(" Bands "))
        .column_spacing(2);

    f.render_widget(table, area);
}

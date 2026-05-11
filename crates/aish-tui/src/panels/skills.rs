use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Row, Table},
    Frame,
};

use crate::app::App;

pub fn render_skills(f: &mut Frame, app: &App, area: Rect) {
    let rows: Vec<Row> = app
        .skills
        .iter()
        .map(|s| {
            let status = if s.loaded { "loaded" } else { "not loaded" };
            let _ = if s.loaded {
                Color::Green
            } else {
                Color::DarkGray
            };

            Row::new(vec![
                s.name.clone(),
                s.description.clone(),
                status.to_string(),
                s.call_count.to_string(),
            ])
            .style(Style::default().fg(Color::White))
        })
        .collect();

    let widths = [
        ratatui::layout::Constraint::Length(18),
        ratatui::layout::Constraint::Percentage(40),
        ratatui::layout::Constraint::Length(12),
        ratatui::layout::Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["Name", "Description", "Status", "Calls"]).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(Block::default().borders(Borders::ALL).title(" Skills "))
        .column_spacing(2);

    f.render_widget(table, area);
}

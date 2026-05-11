use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Row, Table},
    Frame,
};

use crate::app::App;

pub fn render_models(f: &mut Frame, app: &App, area: Rect) {
    let _ = vec![
        Line::from(Span::styled(
            " Global Default: claude-sonnet-4-6    [Enter: switch] [G: set global]",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
    ];

    let rows: Vec<Row> = app
        .models
        .iter()
        .map(|m| {
            let current_mark = if m.is_current { "●" } else { " " };
            let ctx = m
                .context_window
                .map(|c| format!("{}k", c / 1000))
                .unwrap_or_else(|| "--".into());

            let style = if m.is_current {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            Row::new(vec![
                current_mark.to_string(),
                m.id.clone(),
                m.provider.clone(),
                ctx,
            ])
            .style(style)
        })
        .collect();

    let widths = [
        ratatui::layout::Constraint::Length(1),
        ratatui::layout::Constraint::Percentage(40),
        ratatui::layout::Constraint::Percentage(30),
        ratatui::layout::Constraint::Percentage(30),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["", "Model", "Provider", "Context"]).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(Block::default().borders(Borders::ALL).title(" Models "))
        .column_spacing(2);

    f.render_widget(table, area);
}

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Row, Table},
    Frame,
};

use crate::app::App;

pub fn render_mcp(f: &mut Frame, app: &App, area: Rect) {
    let rows: Vec<Row> = app
        .mcp_servers
        .iter()
        .map(|s| {
            let (status_icon, _) = match s.status.as_str() {
                "connected" => ("●", Color::Green),
                "connecting" => ("◌", Color::Yellow),
                _ => ("○", Color::Red),
            };

            let style = if s.status == "connected" {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            };

            Row::new(vec![
                format!("{} {}", status_icon, s.name),
                s.status.clone(),
                s.tools.to_string(),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        ratatui::layout::Constraint::Percentage(50),
        ratatui::layout::Constraint::Percentage(25),
        ratatui::layout::Constraint::Percentage(25),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["Server", "Status", "Tools"])
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        )
        .block(Block::default().borders(Borders::ALL).title(" MCP Servers "))
        .column_spacing(2);

    f.render_widget(table, area);
}

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Row, Table},
    Frame,
};

use crate::app::App;

pub fn render_permissions(f: &mut Frame, app: &App, area: Rect) {
    let _ = vec![
        Line::from(Span::styled(
            format!(
                " Agent: [local/claude-code]    Audit Log: {} changes    [Space: cycle]",
                app.permissions.len()
            ),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
    ];

    let rows: Vec<Row> = app
        .permissions
        .iter()
        .map(|p| {
            let _perm_color = match p.permit {
                aish_core::types::Permit::Allow => Color::Green,
                aish_core::types::Permit::Deny => Color::Red,
                aish_core::types::Permit::Ask => Color::Yellow,
            };

            let last = p
                .last_changed
                .as_deref()
                .unwrap_or("-- (default)");

            Row::new(vec![
                p.tool.clone(),
                p.permit.to_string(),
                p.description.clone(),
                last.to_string(),
            ])
            .style(Style::default().fg(Color::White))
        })
        .collect();

    let widths = [
        ratatui::layout::Constraint::Length(14),
        ratatui::layout::Constraint::Length(8),
        ratatui::layout::Constraint::Percentage(40),
        ratatui::layout::Constraint::Percentage(20),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["Tool", "Permit", "Description", "Last Changed"])
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        )
        .block(Block::default().borders(Borders::ALL).title(" Permissions "))
        .column_spacing(1);

    f.render_widget(table, area);
}

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::App;

pub fn render_activity(f: &mut Frame, app: &App, area: Rect) {
    if app.activity_log.is_empty() {
        let empty = Paragraph::new("No activity yet.\n\nTool calls will appear here as agents work.")
            .block(Block::default().borders(Borders::ALL).title(" Activity "))
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app
        .activity_log
        .iter()
        .map(|entry| {
            let time = entry.timestamp.format("%H:%M:%S").to_string();
            let status_icon = if entry.success { "✓" } else { "✗" };
            let status_color = if entry.success {
                Color::Green
            } else {
                Color::Red
            };

            let agent_name = entry.agent.to_string();
            let mut spans = vec![
                Span::styled(time, Style::default().fg(Color::DarkGray)),
                Span::raw("  "),
                Span::styled(
                    agent_name,
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw("  "),
            ];

            // Task ID if available
            if let Some(ref tid) = entry.task_id {
                spans.push(Span::styled(
                    format!("{}  ", tid),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            // Tool name with color
            let tool_color = match entry.tool.as_str() {
                "Read" => Color::Green,
                "Write" => Color::Yellow,
                "Edit" => Color::Yellow,
                "Grep" => Color::Blue,
                "Glob" => Color::Blue,
                "Bash" => Color::Magenta,
                "Agent" => Color::Cyan,
                _ => Color::Gray,
            };
            spans.push(Span::styled(
                format!("{:<8}", entry.tool),
                Style::default().fg(tool_color).add_modifier(Modifier::BOLD),
            ));

            // Args
            spans.push(Span::styled(
                format!("{:<24}", entry.args),
                Style::default().fg(Color::White),
            ));

            // Status
            spans.push(Span::styled(
                format!(" {} ", status_icon),
                Style::default().fg(status_color),
            ));

            // Duration
            let dur = if entry.duration_ms >= 1000 {
                format!("{:.1}s", entry.duration_ms as f64 / 1000.0)
            } else {
                format!("{}ms", entry.duration_ms)
            };
            spans.push(Span::styled(dur, Style::default().fg(Color::DarkGray)));

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Activity ")
                .title_bottom(" [/ search] [c: clear] [→ details] "),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_widget(list, area);
}

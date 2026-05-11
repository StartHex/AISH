use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::App;
use aish_core::types::TaskStatus;

pub fn render_tasks(f: &mut Frame, app: &App, area: Rect) {
    if app.tasks.is_empty() {
        let empty = Paragraph::new("No tasks yet.\n\nSubmit a task with `aish exec` or use the command bar.")
            .block(Block::default().borders(Borders::ALL).title(" Tasks "))
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = app
        .tasks
        .iter()
        .map(|task| {
            let (status_icon, status_color) = match &task.status {
                TaskStatus::Running { .. } => ("●", Color::Yellow),
                TaskStatus::Done { .. } => ("✓", Color::Green),
                TaskStatus::Failed { .. } => ("✗", Color::Red),
                TaskStatus::Cancelled => ("⊘", Color::DarkGray),
                TaskStatus::Queued => ("○", Color::Gray),
            };

            let mut lines = vec![];
            let mut first_line = vec![
                Span::styled(status_icon, Style::default().fg(status_color)),
                Span::raw(" "),
                Span::styled(
                    &task.prompt,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ];

            // Add progress bar for running tasks
            if let Some(progress) = task.progress {
                let bar_width = 10;
                let filled = (progress * bar_width as f32) as usize;
                let empty = bar_width - filled;
                first_line.push(Span::raw("  "));
                first_line.push(Span::styled(
                    format!(
                        "{}{} {}%",
                        "█".repeat(filled),
                        "░".repeat(empty),
                        (progress * 100.0) as u8
                    ),
                    Style::default().fg(Color::Yellow),
                ));
            }

            let meta = format!(
                "  {}  {}",
                task.duration.as_deref().unwrap_or("--"),
                task.id
            );
            first_line.push(Span::styled(meta, Style::default().fg(Color::DarkGray)));

            lines.push(Line::from(first_line));

            // Agent line
            lines.push(Line::from(vec![
                Span::raw("   ▶ "),
                Span::styled(task.agent.to_string(), Style::default().fg(Color::Cyan)),
            ]));

            ListItem::new(lines)
        })
        .collect();

    let task_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" Tasks "))
        .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_widget(task_list, area);
}

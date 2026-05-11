use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::app::App;
use aish_core::types::AgentStatus;

pub fn render_agents(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .agents
        .iter()
        .enumerate()
        .map(|(i, agent)| {
            let (status_icon, status_color) = match agent.status {
                AgentStatus::Online { .. } => ("●", Color::Green),
                AgentStatus::Busy { .. } => ("◉", Color::Yellow),
                AgentStatus::Degraded { .. } => ("⚠", Color::Yellow),
                AgentStatus::Offline { .. } => ("○", Color::DarkGray),
                AgentStatus::Connecting => ("◌", Color::Cyan),
            };

            let is_selected = i == app.agent_selected;
            let base_style = if is_selected {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::Gray)
            };

            let mut lines = vec![Line::from(vec![
                Span::styled(status_icon, Style::default().fg(status_color)),
                Span::raw(" "),
                Span::styled(&agent.alias, base_style.add_modifier(Modifier::BOLD)),
            ])];

            // Status line
            let mut status_line = vec![Span::raw("  ")];
            match &agent.status {
                AgentStatus::Online { uptime: _, model: _ } => {
                    status_line.push(Span::styled(
                        format!("Online · uptime {}", agent.uptime),
                        Style::default().fg(Color::Green),
                    ));
                }
                AgentStatus::Busy {
                    current_task: _,
                    progress: _,
                    model: _,
                } => {
                    if let Some(ref task) = agent.current_task {
                        status_line.push(Span::styled(
                            format!("Busy: {}", task),
                            Style::default().fg(Color::Yellow),
                        ));
                    } else {
                        status_line.push(Span::styled("Busy", Style::default().fg(Color::Yellow)));
                    }
                }
                AgentStatus::Degraded {
                    model: _,
                    reason: _,
                } => {
                    status_line.push(Span::styled("Degraded", Style::default().fg(Color::Yellow)));
                }
                AgentStatus::Offline { since: _ } => {
                    status_line.push(Span::styled(
                        format!("Offline · since {}", agent.uptime),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                AgentStatus::Connecting => {
                    status_line.push(Span::styled(
                        "Connecting...",
                        Style::default().fg(Color::Cyan),
                    ));
                }
            }
            lines.push(Line::from(status_line));

            // Model line
            lines.push(Line::from(vec![
                Span::raw("  Model: "),
                Span::styled(&agent.model, Style::default().fg(Color::Cyan)),
            ]));

            // Token line
            let tokens_str = format!(
                "  Tokens: {}k in / {}k out",
                agent.tokens_input / 1000,
                agent.tokens_output / 1000
            );
            lines.push(Line::from(Span::raw(tokens_str)));

            // Progress bar for busy agents
            if let Some(progress) = agent.progress {
                let bar_width = 20;
                let filled = (progress * bar_width as f32) as usize;
                let empty = bar_width - filled;
                let bar = format!(
                    "  Progress: {}{} {}%",
                    "█".repeat(filled),
                    "░".repeat(empty),
                    (progress * 100.0) as u8
                );
                lines.push(Line::from(Span::styled(bar, Style::default().fg(Color::Yellow))));
            }

            // Spacer
            lines.push(Line::from(""));

            ListItem::new(lines)
        })
        .collect();

    let add_item = ListItem::new(Line::from(Span::styled(
        "[ + Add Agent ]",
        Style::default().fg(Color::DarkGray),
    )));
    let items: Vec<ListItem> = items.into_iter().chain(std::iter::once(add_item)).collect();

    let list = List::new(items)
        .block(Block::default().title(" Agents ").borders(Borders::ALL))
        .highlight_style(Style::default().bg(Color::DarkGray));

    f.render_widget(list, area);
}

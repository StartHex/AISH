use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, FanOutStrategy};

pub fn render_fanout(f: &mut Frame, app: &App, area: Rect) {
    // Layer selector
    let layer_hint = Line::from(Span::styled(
        " [1.Execute│2.Compare│3.Split│4.Extract]    [Tab: next view]",
        Style::default().fg(Color::DarkGray),
    ));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    f.render_widget(Paragraph::new(layer_hint), chunks[0]);

    if let Some(ref group) = app.fan_out {
        // Strategy line
        let strategy = match group.strategy {
            FanOutStrategy::Parallel => "Parallel",
            FanOutStrategy::Sequential => "Sequential",
            FanOutStrategy::Race => "Race",
            FanOutStrategy::Vote => "Vote",
        };
        let info = format!(
            " Mode: [{}]  Prompt: \"{}\"  Model: [claude-sonnet-4-6]",
            strategy, group.prompt
        );
        f.render_widget(
            Paragraph::new(Span::styled(info, Style::default().fg(Color::Gray))),
            chunks[1],
        );

        // Targets
        let target_text: Vec<Line> = group
            .targets
            .iter()
            .map(|t| {
                let check = if t.selected { "☑" } else { "☐" };
                let online = if t.online {
                    "(online)"
                } else {
                    "(offline — skipped)"
                };
                let status_color = if t.online {
                    Color::Green
                } else {
                    Color::DarkGray
                };
                Line::from(vec![
                    Span::raw(format!(" {} {} ", check, t.agent)),
                    Span::styled(online, Style::default().fg(status_color)),
                ])
            })
            .collect();

        f.render_widget(
            Paragraph::new(target_text)
                .block(Block::default().borders(Borders::ALL).title(" Targets ")),
            chunks[2],
        );

        // Results
        let results: Vec<ListItem> = group
            .targets
            .iter()
            .filter_map(|t| {
                t.result.as_ref().map(|r| {
                    let (icon, color) = match r.status.as_str() {
                        s if s.starts_with("Done") => ("✓", Color::Green),
                        s if s.starts_with("Error") => ("✗", Color::Red),
                        s if s.starts_with("Running") => ("⏳", Color::Yellow),
                        _ => ("?", Color::Gray),
                    };
                    let lines = vec![Line::from(vec![
                        Span::styled(
                            format!("{} {}  ", icon, t.agent),
                            Style::default().fg(color),
                        ),
                        Span::styled(
                            &r.status,
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        ),
                        Span::raw("  "),
                        Span::styled(&r.summary, Style::default().fg(Color::Gray)),
                    ])];
                    ListItem::new(lines)
                })
            })
            .collect();

        let result_list = List::new(results)
            .block(Block::default().borders(Borders::ALL).title(" Results "))
            .style(Style::default().fg(Color::White));

        f.render_widget(result_list, chunks[3]);

        // Footer
        let footer = Span::styled(
            " [Enter: execute]  [Space: toggle agent]  [s: switch strategy]",
            Style::default().fg(Color::DarkGray),
        );
        f.render_widget(Paragraph::new(footer), chunks[4]);
    }
}

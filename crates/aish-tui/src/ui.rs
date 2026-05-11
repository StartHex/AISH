//! UI rendering — assembles the full TUI layout.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Tabs},
    Frame,
};

use crate::app::{App, Tab};
use crate::panels;

pub fn render(f: &mut Frame, app: &App) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Status bar
            Constraint::Min(0),    // Main content
            Constraint::Length(1), // Command bar
        ])
        .split(f.area());

    render_status_bar(f, app, main_chunks[0]);
    render_main_area(f, app, main_chunks[1]);
    render_command_bar(f, app, main_chunks[2]);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let online = app
        .agents
        .iter()
        .filter(|a| !matches!(a.status, aish_core::types::AgentStatus::Offline { .. }))
        .count();
    let busy = app
        .agents
        .iter()
        .filter(|a| matches!(a.status, aish_core::types::AgentStatus::Busy { .. }))
        .count();
    let total_tokens: u64 = app.agents.iter().map(|a| a.tokens_input + a.tokens_output).sum();

    let text = format!(
        " AISH v0.1 · {} online ({} busy) · {} tasks · {}k tokens · F1:Help ",
        online,
        busy,
        app.tasks.len(),
        total_tokens / 1000,
    );

    let bar = Paragraph::new(text)
        .style(Style::default().fg(Color::Black).bg(Color::DarkGray));

    f.render_widget(bar, area);
}

fn render_main_area(f: &mut Frame, app: &App, area: Rect) {
    let main_split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(28), Constraint::Percentage(72)])
        .split(area);

    // Left panel: Agents
    panels::render_agents(f, app, main_split[0]);

    // Right panel: Tab content
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(main_split[1]);

    render_tab_bar(f, app, right_chunks[0]);
    render_tab_content(f, app, right_chunks[1]);
}

fn render_tab_bar(f: &mut Frame, app: &App, area: Rect) {
    let tab_labels: Vec<Line> = Tab::all()
        .iter()
        .map(|tab| {
            let style = if *tab == app.selected_tab {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::from(Span::styled(format!(" {} ", tab.label()), style))
        })
        .collect();

    let tabs = Tabs::new(tab_labels)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(Style::default().fg(Color::Yellow));

    f.render_widget(tabs, area);
}

fn render_tab_content(f: &mut Frame, app: &App, area: Rect) {
    match app.selected_tab {
        Tab::Tasks => panels::render_tasks(f, app, area),
        Tab::Activity => panels::render_activity(f, app, area),
        Tab::Models => panels::render_models(f, app, area),
        Tab::Permissions => panels::render_permissions(f, app, area),
        Tab::Skills => panels::render_skills(f, app, area),
        Tab::Mcp => panels::render_mcp(f, app, area),
        Tab::Tokens => panels::render_tokens(f, app, area),
        Tab::FanOut => panels::render_fanout(f, app, area),
        Tab::Band => panels::render_bands(f, app, area),
    }
}

fn render_command_bar(f: &mut Frame, app: &App, area: Rect) {
    let text = if app.command_input.is_empty() {
        ":fan-out \"review the auth module\" --all --model claude-sonnet-4-6"
    } else {
        &app.command_input
    };

    let bar = Paragraph::new(format!(":{}", text))
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));

    f.render_widget(bar, area);
}

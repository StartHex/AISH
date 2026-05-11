use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
    Frame,
};

use crate::app::App;

pub fn render_tokens(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Window switcher hint
    let window_hint = Line::from(Span::styled(
        " [T] Today  [M] Month  [A] All    Window: Today",
        Style::default().fg(Color::DarkGray),
    ));
    f.render_widget(
        Paragraph::new(window_hint),
        chunks[0],
    );

    if let Some(ref summary) = app.token_summary {
        // Summary cards
        let cards_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(34),
            ])
            .split(chunks[1]);

        // Input card
        let input_card = Paragraph::new(format!("\n  Input\n  {}k tokens", summary.total_input / 1000))
            .block(Block::default().borders(Borders::ALL).title(" In "))
            .style(Style::default().fg(Color::Cyan))
            .centered();
        f.render_widget(input_card, cards_area[0]);

        // Output card
        let output_card = Paragraph::new(format!("\n  Output\n  {}k tokens", summary.total_output / 1000))
            .block(Block::default().borders(Borders::ALL).title(" Out "))
            .style(Style::default().fg(Color::Magenta))
            .centered();
        f.render_widget(output_card, cards_area[1]);

        // No pricing — just ratio
        let ratio = if summary.total_output > 0 {
            format!("{:.2}:1", summary.total_input as f64 / summary.total_output as f64)
        } else {
            "--".into()
        };
        let ratio_card = Paragraph::new(format!("\n  In/Out Ratio\n  {}", ratio))
            .block(Block::default().borders(Borders::ALL).title(" Ratio "))
            .style(Style::default().fg(Color::Yellow))
            .centered();
        f.render_widget(ratio_card, cards_area[2]);

        // By-model table (below cards, but we're limited on space so put it under)
        // Since we already used all space, the model breakdown is embedded
    }

    // Model breakdown
    if let Some(ref summary) = app.token_summary {
        let model_rows: Vec<Row> = summary
            .by_model
            .iter()
            .map(|m| {
                Row::new(vec![
                    m.model.clone(),
                    format!("{}k", m.input / 1000),
                    format!("{}k", m.output / 1000),
                    format!("{}k", (m.input + m.output) / 1000),
                ])
                .style(Style::default().fg(Color::White))
            })
            .collect();

        // Fix: render model table in remaining space
        let _model_table = Table::new(
            model_rows,
            [
                Constraint::Percentage(40),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
            ],
        )
        .header(
            Row::new(vec!["Model", "Input", "Output", "Total"])
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        )
        .block(Block::default().borders(Borders::ALL));
    }
}

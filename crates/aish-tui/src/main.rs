//! AISH TUI — ratatui-based terminal interface for AI agent shell management.

mod app;
mod panels;
mod ui;

use app::App;
use crossterm::{
    event::{self, Event as CEvent, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::Terminal;
use std::io::stdout;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();

    // Ensure alternate screen is entered before starting
    execute!(stdout, EnterAlternateScreen, crossterm::cursor::Hide)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let app = App::new();
    let result = run_event_loop(&mut terminal, app).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::cursor::Show
    )?;

    result
}

async fn run_event_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> anyhow::Result<()> {
    let tick_rate = Duration::from_millis(100);

    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        if app.should_quit {
            break;
        }

        if event::poll(tick_rate)? {
            if let CEvent::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => app.should_quit = true,
                        KeyCode::Esc => app.should_quit = true,
                        KeyCode::Tab => app.next_tab(),
                        KeyCode::BackTab => app.prev_tab(),
                        KeyCode::Char('j') | KeyCode::Down => {
                            if app.selected_tab == app::Tab::Tasks {
                                app.task_selected =
                                    (app.task_selected + 1).min(app.tasks.len().saturating_sub(1));
                            }
                            if app.selected_tab == app::Tab::Activity {
                                app.activity_selected = (app.activity_selected + 1)
                                    .min(app.activity_log.len().saturating_sub(1));
                            }
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if app.selected_tab == app::Tab::Tasks {
                                app.task_selected = app.task_selected.saturating_sub(1);
                            }
                            if app.selected_tab == app::Tab::Activity {
                                app.activity_selected = app.activity_selected.saturating_sub(1);
                            }
                        }
                        KeyCode::Char('1') => app.selected_tab = app::Tab::Tasks,
                        KeyCode::Char('2') => app.selected_tab = app::Tab::Activity,
                        KeyCode::Char('3') => app.selected_tab = app::Tab::Models,
                        KeyCode::Char('4') => app.selected_tab = app::Tab::Permissions,
                        KeyCode::Char('5') => app.selected_tab = app::Tab::Skills,
                        KeyCode::Char('6') => app.selected_tab = app::Tab::Mcp,
                        KeyCode::Char('7') => app.selected_tab = app::Tab::Tokens,
                        KeyCode::Char('8') => app.selected_tab = app::Tab::FanOut,
                        KeyCode::Char('9') => app.selected_tab = app::Tab::Band,
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}

//! Interactive Terminal User Interface
//!
//! Provides a modern TUI for device management using ratatui.

mod app;
mod widgets;

use anyhow::Result;
use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use std::time::Duration;
use usboverssh_core::Config;

/// Run the TUI
pub async fn run(connect_on_start: bool, config: Config) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(config);

    if connect_on_start {
        app.connect_all_hosts().await;
    }

    // Initial device refresh
    app.refresh_devices().await;

    // Main loop
    let result = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

/// Main application loop
async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        // Draw UI
        terminal.draw(|f| {
            widgets::render(f, app);
        })?;

        // Handle input with timeout for refresh
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            if app.is_popup_open() {
                                app.close_popup();
                            } else {
                                return Ok(());
                            }
                        }

                        KeyCode::Char('?') | KeyCode::F(1) => {
                            app.toggle_help();
                        }

                        KeyCode::Tab => {
                            app.next_pane();
                        }

                        KeyCode::BackTab => {
                            app.prev_pane();
                        }

                        KeyCode::Up | KeyCode::Char('k') => {
                            app.select_prev();
                        }

                        KeyCode::Down | KeyCode::Char('j') => {
                            app.select_next();
                        }

                        KeyCode::Enter => {
                            app.activate_selected().await;
                        }

                        KeyCode::Char('r') | KeyCode::F(5) => {
                            app.refresh_devices().await;
                        }

                        KeyCode::Char('a') => {
                            app.attach_selected().await;
                        }

                        KeyCode::Char('d') => {
                            app.detach_selected().await;
                        }

                        KeyCode::Char('c') => {
                            app.open_connect_dialog();
                        }

                        KeyCode::Char('h') => {
                            app.open_hosts_panel();
                        }

                        KeyCode::Char('s') => {
                            app.toggle_status_panel();
                        }

                        _ => {}
                    }
                }
            }
        }

        // Periodic refresh
        app.tick().await;
    }
}

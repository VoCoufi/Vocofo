use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

mod file_operation;
mod ui;
mod context;
mod render;
mod event_handler;

use crate::context::Context;

// Application error type for better error handling
type AppResult<T> = Result<T, Box<dyn std::error::Error>>;

fn main() -> AppResult<()> {
    // Setup terminal
    let mut terminal = setup_terminal()?;

    // Create application state
    let mut context = Context::new();

    // Run the main application loop
    let result = run_app(&mut terminal, &mut context);

    // Restore terminal
    restore_terminal()?;

    // Return any errors that might have occurred during the application run
    result
}

/// Set up the terminal for the TUI application
fn setup_terminal() -> AppResult<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableFocusChange,
        EnableMouseCapture
    )?;

    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;

    Ok(terminal)
}

/// Restore the terminal to its original state
fn restore_terminal() -> AppResult<()> {
    disable_raw_mode()?;
    execute!(
        io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableFocusChange
    )?;

    Ok(())
}

/// Run the application main loop
fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    context: &mut Context) -> AppResult<()> {
    const POLL_TIMEOUT: Duration = Duration::from_millis(50);

    loop {
        // Render the UI
        terminal.draw(|frame| ui::ui(frame, context).expect("REASON"))?;

        // Handle events and break loop if exit is requested
        if handle_events(context, POLL_TIMEOUT)? {
            break;
        }
    }

    Ok(())
}

/// Handle terminal events
fn handle_events(context: &mut Context, timeout: Duration) -> AppResult<bool> {
    // Check if there are any events available
    if event::poll(timeout)? {
        if let Event::Key(key) = event::read()? {
            // Only process press events
            if key.kind == KeyEventKind::Press {
                // Determine which event handler to use based on application state
                match (context.get_popup().unwrap(), context.get_confirm_popup().unwrap()) {
                    (false, false) => event_handler::handle_main_event(context, key),
                    (true, _) => event_handler::handle_popup_event(context, key),
                    (_, true) => event_handler::handle_confirm_popup_event(context, key),
                }?;
            }
        }
    }

    // Return whether we should exit the application
    Ok(context.get_exit().unwrap())
}
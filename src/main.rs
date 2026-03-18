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
mod messages_enum;
mod background_op;

use crate::context::Context;

// Application error type for better error handling
type AppResult<T> = Result<T, Box<dyn std::error::Error>>;

fn main() -> AppResult<()> {
    // Setup terminal
    let mut terminal = setup_terminal()?;

    // Create an application state
    let mut context = Context::new()?;

    // Run the main application loop
    let result = run_app(&mut terminal, &mut context);

    // Restore terminal
    restore_terminal()?;

    // Return any errors that might have occurred during the application run
    result
}

/// Sets up and configures the terminal for a TUI (Text User Interface) application.
///
/// This function performs the following tasks:
/// - Enables raw mode to allow finer control of terminal input and output.
/// - Redirects the output to an alternate screen, enabling a clean slate for the TUI.
/// - Enables focus changes and mouse capture for better interaction support.
/// - Initializes a new `Terminal` instance with a `CrosstermBackend`.
///
/// # Returns
/// This function returns a `Result` containing:
/// - `Ok` with a `Terminal<CrosstermBackend<io::Stdout>>` instance if successful.
/// - An error of type `AppResult` if setup fails at any point.
///
/// # Errors
/// This function propagates errors that can occur during:
/// - Enabling raw mode with `enable_raw_mode()`.
/// - Executing terminal commands like entering the alternate screen, enabling focus changes, or mouse capture.
/// - Initializing the terminal backend.
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

/// Restores the terminal to its normal state by disabling raw mode and executing necessary commands.
///
/// This function performs the following operations:
/// 1. Disables raw mode, which restores the terminal's default line buffering and input behavior.
/// 2. Exits the alternate screen mode, restoring the main screen content.
/// 3. Disables mouse capture, restoring the default terminal mouse behavior.
/// 4. Disables focus change events.
///
/// # Returns
/// - `Ok(())` on successful restoration of the terminal state.
/// - An `AppResult` containing an error if any of the operations fail.
///
/// This function is typically used to revert the terminal state after an application has used raw mode
/// or alternate screen mode for its functionality.
///
/// # Errors
/// Returns an error if disabling raw mode or executing the terminal commands fails.
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

    // Populate both panels with directory listings
    file_operation::list_children(&mut context.panels[0])
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    file_operation::list_children(&mut context.panels[1])
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    loop {
        // Render the UI
        let mut render_error: Option<Box<dyn std::error::Error>> = None;
        terminal.draw(|frame| {
            if let Err(e) = ui::ui(frame, context) {
                render_error = Some(e);
            }
        })?;
        if let Some(e) = render_error {
            return Err(e);
        }

        // Check for completed background operations
        if let Some(result) = context.check_operation() {
            match result.result {
                Ok(()) => {
                    context.invalidate_all_caches();
                    // Clamp selection state to valid range after items may have changed
                    for panel in &mut context.panels {
                        // Force refresh so items are up to date
                        let _ = file_operation::list_children(panel);
                        if panel.state > 0 && panel.state >= panel.items.len() {
                            panel.state = panel.items.len().saturating_sub(1);
                        }
                    }
                    context.set_status_message(&format!("{} done", result.description.trim_end_matches("...")));
                    if result.clear_clipboard {
                        context.copy_path = String::default();
                    }
                }
                Err(e) => {
                    context.set_status_message(&format!("Failed: {}", e));
                }
            }
        }

        // Advance spinner for progress indicator
        context.spinner_tick = context.spinner_tick.wrapping_add(1);

        // Handle events and break the loop if exit is requested
        if handle_events(context, POLL_TIMEOUT)? {
            break;
        }
    }

    Ok(())
}

/// Handles input events for the application, delegating to appropriate event handlers based on the current application state.
///
/// This function checks for any user input events within a given timeout duration. If an event is detected,
/// it processes the event accordingly using the associated event handler. It delegates the event handling
/// based on the application's current state (main, popup, or confirmation popup).
///
/// # Arguments
///
/// * `context` - A mutable reference to the `Context` struct holding the application's state and data.
/// * `timeout` - A `Duration` object specifying how long to wait for an event to occur before timing out.
///
/// # Returns
///
/// Returns an `AppResult<bool>`:
/// * `true` if the application should exit.
/// * `false` otherwise.
///
/// # Errors
///
/// Returns an error if there is a failure in polling or reading events, or if the event handlers return an error.
///
/// # Event Handling Flow
///
/// * If no events are ready within the specified timeout, the function does nothing and returns immediately.
/// * If an event is detected:
///   * The function processes only key press events (`KeyEventKind::Press`).
///   * Depending on the application state:
///     - If no popup or confirmation popup is active, the event is passed to the main event handler: `handle_main_event`.
///     - If a popup is active, the event is passed to the popup event handler: `handle_popup_event`.
///     - If a confirmation popup is active, the event is passed to the confirmation popup event handler: `handle_confirm_popup_event`.
fn handle_events(context: &mut Context, timeout: Duration) -> AppResult<bool> {
    // Check if there are any events available
    if event::poll(timeout)? {
        if let Event::Key(key) = event::read()? {
            // Only process press events
            if key.kind == KeyEventKind::Press {
                // Determine which event handler to use based on the application state
                match context.ui_state {
                    context::UiState::Normal => event_handler::handle_main_event(context, key),
                    context::UiState::CreatePopup => event_handler::handle_popup_event(context, key),
                    context::UiState::ConfirmDelete => event_handler::handle_confirm_popup_event(context, key),
                    context::UiState::RenamePopup => event_handler::handle_rename_popup_event(context, key),
                }?;
            }
        }
    }

    // Return whether we should exit the application
    Ok(context.get_exit().unwrap_or(false))
}
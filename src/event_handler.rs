use crate::context::{Context, UiState};
use crate::file_operation;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Result type for event handler operations
type EventResult = Result<(), Box<dyn std::error::Error>>;

/// Handles the main event loop for the application by processing key events and updating the application state.
///
/// # Arguments
///
/// * `context` - A mutable reference to the `Context` object, which maintains the state and functionality of the application.
/// * `key_event` - The `KeyEvent` that represents the keyboard input provided by the user.
///
/// # Behavior
///
/// The function processes various key events and performs the following actions:
/// - `KeyCode::Char('q')` or `KeyCode::Esc`: Exits the application by calling `context.set_exit()`.
/// - `KeyCode::Enter`: Opens the selected item by calling `context.open_item()`.
/// - `KeyCode::Tab`: Opens a directory through a helper function, `file_operation::open_dir`.
/// - `KeyCode::Down`: Moves the selection down if there are more items below the current state, using `context.increment_state()`.
/// - `KeyCode::Up`: Moves the selection up if the current state is not at the top, using `context.decrease_state()`.
/// - `KeyCode::Char('p')`: Changes the UI state to `CreatePopup` by calling `context.set_ui_state(UiState::CreatePopup)`.
/// - `KeyCode::Char('c')`: Placeholder for a copy functionality (currently a TODO).
/// - `KeyCode::Char('d')`: Changes the UI state to confirm deletion by calling `context.set_ui_state(UiState::ConfirmDelete)`.
/// - Any other keys will be ignored and have no effect.
///
/// # Returns
///
/// * `EventResult` - An `Ok` result is returned to indicate the event was handled.
///
/// # Notes
///
/// This function serves as the main entry point for handling user input within the context of the application workflow. It relies on the state and functionality provided by the `Context` object, as well as external file operation utilities where applicable.
pub fn handle_main_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    match (key_event.code, key_event.modifiers) {
        (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => {
            context.set_exit();
        }
        (KeyCode::Enter, _) => {
            context.open_item();
        }
        (KeyCode::Tab, _) => {
            file_operation::open_dir(context)?;
        }
        (KeyCode::Down, _) => {
            if context.items.len() > context.state + 1 {
                context.increment_state();
            }
        }
        (KeyCode::Up, _) => {
            if context.state > 0 {
                context.decrease_state();
            }
        }
        (KeyCode::Char('p'), _) => {
            context.set_ui_state(UiState::CreatePopup);
        }
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            context.set_copy_path();
            context.set_status_message("Copied to clipboard");
        }
        (KeyCode::Char('v'), KeyModifiers::CONTROL) => {
            if context.get_copy_path().is_empty() {
                context.set_status_message("Nothing to paste — copy a file first");
                return Ok(());
            }
            file_operation::copy_file(context)?;
            context.set_status_message("Pasted successfully");
        }
        (KeyCode::Char('d'), _) => {
            context.set_ui_state(UiState::ConfirmDelete);
        }
        _ => {}
    }

    // Update preview after any navigation or state change
    context.update_preview();

    Ok(())
}

/// Handles the key events for the popup UI state.
///
/// This function is responsible for processing user input when a popup is active. 
/// It interprets the different key events and modifies the program state accordingly.
///
/// # Arguments
///
/// * `context` - A mutable reference to the application context, which contains the current state of the UI, 
///               input buffer, and other contextual information.
/// * `key_event` - The key event triggered by the user, representing a keystroke or action.
///
/// # KeyEvent Handling
///
/// * `KeyCode::Backspace` - Removes the last character from the `input` field in the context, if not empty.
/// * `KeyCode::Enter` - Calls `file_operation::handle_create_directory` to trigger directory creation based on the input. 
///                      Any errors during this operation will propagate up the stack.
/// * `KeyCode::Esc` - Changes the UI state back to `Normal`, which exits the popup mode.
/// * `KeyCode::Char(c)` - Appends the typed character `c` to the `input` field in the context.
/// * Any other key events are ignored.
///
/// # Return
///
/// Returns an `EventResult` which is an alias for `Result<(), Error>`. 
/// If the `KeyCode::Enter` operation fails, the error is propagated via the `?` operator.
///
/// # Errors
///
/// This function may return an error if `file_operation::handle_create_directory` fails when handling the `KeyCode::Enter`.
pub fn handle_popup_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    match key_event.code {
        KeyCode::Backspace => {
            if !context.input.is_empty() {
                context.input.pop();
            }
        }
        KeyCode::Enter => {
            file_operation::handle_create_directory(context)?;
        }
        KeyCode::Esc => {
            context.set_ui_state(UiState::Normal);
        }
        KeyCode::Char(c) => {
            context.input.push(c);
        }
        _ => {}
    }
    Ok(())
}

/// Handles user input events for the confirmation popup.
///
/// This function processes key events related to the confirmation popup UI and executes the
/// associated actions based on the key pressed. It updates the UI state and executes
/// operations such as file deletion or navigation within the popup.
///
/// # Parameters
/// - `context`: A mutable reference to the application's `Context` object, containing relevant
///   state information and methods for updating the UI.
/// - `key_event`: A `KeyEvent` representing the key event triggered by the user.
///
/// # Key Bindings
/// - **'y'**: Confirms the operation (e.g., deletes a file), sets the UI state back to `UiState::Normal`, 
///   and sets the confirm button state.
/// - **'n' or Escape**: Cancels the operation and resets the UI state to `UiState::Normal`.
/// - **Left Arrow**: Selects the "Yes" button in the confirmation popup.
/// - **Right Arrow**: Selects the "No" button in the confirmation popup.
/// - **Enter**: Executes the currently selected action. If "Yes" is selected, it performs the delete operation
///   and resets the confirmation button's state. Regardless, the UI state is reset to `UiState::Normal`.
///
/// # Returns
/// - `EventResult`: Returns a result indicating the outcome of the event handling. 
///   If a file deletion operation is triggered, it propagates any error from the `file_operation::handle_delete_operation` function.
///
/// # Errors
/// - Returns an error if the file deletion operation fails during confirmation.
pub fn handle_confirm_popup_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    match key_event.code {
        KeyCode::Char('y') => {
            context.set_ui_state(UiState::Normal);
            context.set_confirm_button_selected();
            
            // Delete file
            file_operation::handle_delete_operation(context)?
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            context.set_ui_state(UiState::Normal);
        }
        KeyCode::Left => {
            if !context.get_confirm_button_selected().unwrap_or(false) {
                context.set_confirm_button_selected() // Set to true
            }
        }
        KeyCode::Right => {
            if context.get_confirm_button_selected().unwrap_or(false) {
                context.set_confirm_button_selected() // Set to false
            }
        }
        KeyCode::Enter => {
            context.set_ui_state(UiState::Normal);

            if context.get_confirm_button_selected().unwrap_or(false) {
                // Delete a file
                file_operation::handle_delete_operation(context)?;
                context.set_confirm_button_selected()
            }
        }
        _ => {}
    }
    Ok(())
}
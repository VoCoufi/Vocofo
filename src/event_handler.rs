use crate::context::Context;
use crate::file_operation;
use crossterm::event::{KeyCode, KeyEvent};
use std::path::PathBuf;

/// Result type for event handler operations
type EventResult = Result<(), Box<dyn std::error::Error>>;

/// Handle keyboard events in the main view
pub fn handle_main_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    match key_event.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            context.set_exit();
        }
        KeyCode::Enter => {
            context.open_item();
        }
        KeyCode::Tab => {
            file_operation::open_dir(context);
        }
        KeyCode::Down => {
            if context.items.len() > context.state + 1 {
                context.increment_state();
            }
        }
        KeyCode::Up => {
            if context.state > 0 {
                context.decrease_state();
            }
        }
        KeyCode::Char('p') => {
            context.set_popup();
        }
        KeyCode::Char('c') => {
            context.set_confirm_popup();
        }
        KeyCode::Char('d') => {
            file_operation::handle_delete_operation(context)?;
        }
        _ => {}
    }
    Ok(())
}

/// Handle keyboard events in the popup view
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
            context.set_popup();
        }
        KeyCode::Char(c) => {
            context.input.push(c);
        }
        _ => {}
    }
    Ok(())
}

/// Handle keyboard events in the confirmation popup view
pub fn handle_confirm_popup_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    match key_event.code {
        KeyCode::Char('y') => {
            context.set_confirm_popup();
            context.set_confirm_button_selected();
            
            // Delete file
            file_operation::handle_delete_operation(context)?
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            context.set_confirm_popup();
        }
        KeyCode::Left => {
            if !context.get_confirm_button_selected().unwrap() {
                context.set_confirm_button_selected() // Set to true
            }
        }
        KeyCode::Right => {
            if context.get_confirm_button_selected().unwrap() {
                context.set_confirm_button_selected() // Set to false
            }
        }
        KeyCode::Enter => {
            context.set_confirm_popup();
            
            if context.get_confirm_button_selected().unwrap() {
                // Delete file
                file_operation::handle_delete_operation(context)?;
                context.set_confirm_button_selected()
            }
        }
        _ => {}
    }
    Ok(())
}
use crate::background_op;
use crate::context::{Context, UiState};
use crate::file_operation;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Result type for event handler operations
type EventResult = Result<(), Box<dyn std::error::Error>>;

/// Handles the main event loop for the application
pub fn handle_main_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    context.clear_status_message();

    match (key_event.code, key_event.modifiers) {
        (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => {
            context.set_exit();
        }
        (KeyCode::Enter, _) => {
            if let Some(err) = context.active_mut().open_item() {
                context.set_status_message(&err);
            }
            context.active_mut().invalidate_directory_cache();
        }
        (KeyCode::Tab, _) => {
            context.toggle_active_panel();
        }
        (KeyCode::Backspace, _) => {
            if let Some(err) = context.active_mut().navigate_to_parent() {
                context.set_status_message(&err);
            }
        }
        (KeyCode::Down, _) => {
            let panel = context.active_mut();
            if panel.items.len() > panel.state + 1 {
                panel.increment_state();
            }
        }
        (KeyCode::Up, _) => {
            let panel = context.active_mut();
            if panel.state > 0 {
                panel.decrease_state();
            }
        }
        (KeyCode::Char('p'), _) => {
            context.set_ui_state(UiState::CreatePopup);
        }
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            context.set_copy_path();
            context.clipboard_mode = crate::context::ClipboardMode::Copy;
            context.set_status_message("Copied to clipboard");
        }
        (KeyCode::Char('v'), KeyModifiers::CONTROL) => {
            if context.is_operation_running() {
                context.set_status_message("Operation already in progress");
                return Ok(());
            }
            if context.get_copy_path().is_empty() {
                context.set_status_message("Nothing to paste — copy a file first");
                return Ok(());
            }
            match file_operation::resolve_paste_paths(context) {
                Ok((from, to)) => {
                    let name = from.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "item".to_string());
                    let is_cut = context.clipboard_mode == crate::context::ClipboardMode::Cut;
                    let (desc, rx) = if is_cut {
                        let desc = format!("Moving {}...", name);
                        let rx = background_op::spawn_move(from, to, desc.clone());
                        (desc, rx)
                    } else {
                        let desc = format!("Copying {}...", name);
                        let rx = background_op::spawn_copy(from, to, desc.clone());
                        (desc, rx)
                    };
                    context.start_operation(rx, desc);
                }
                Err(e) => {
                    context.set_status_message(&format!("Paste failed: {}", e));
                }
            }
        }
        (KeyCode::Char('x'), KeyModifiers::CONTROL) => {
            context.set_copy_path();
            context.clipboard_mode = crate::context::ClipboardMode::Cut;
            context.set_status_message("Cut to clipboard");
        }
        (KeyCode::Char('d'), _) => {
            context.set_ui_state(UiState::ConfirmDelete);
        }
        (KeyCode::Char('r'), _) => {
            if let Some(item) = context.active().get_selected_item() {
                if item != "../" {
                    let name = item.trim_end_matches('/').to_string();
                    context.set_input(name);
                    context.set_ui_state(UiState::RenamePopup);
                }
            }
        }
        (KeyCode::F(3), _) => {
            context.show_preview = !context.show_preview;
        }
        _ => {}
    }

    // Update preview after any navigation or state change
    if context.show_preview {
        context.active_mut().update_preview();
    }

    Ok(())
}

/// Handles key events for the create folder popup
pub fn handle_popup_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    match key_event.code {
        KeyCode::Backspace => {
            if !context.input.is_empty() {
                context.input.pop();
            }
        }
        KeyCode::Enter => {
            match file_operation::handle_create_directory(context) {
                Ok(_) => {
                    context.active_mut().invalidate_directory_cache();
                    context.set_status_message("Folder created");
                }
                Err(e) => {
                    context.set_ui_state(UiState::Normal);
                    context.set_input(String::default());
                    context.set_status_message(&format!("Create failed: {}", e));
                }
            }
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

/// Handles key events for the rename popup
pub fn handle_rename_popup_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    match key_event.code {
        KeyCode::Backspace => {
            if !context.input.is_empty() {
                context.input.pop();
            }
        }
        KeyCode::Enter => {
            match file_operation::handle_rename(context) {
                Ok(_) => {
                    context.active_mut().invalidate_directory_cache();
                    context.set_status_message("Renamed successfully");
                }
                Err(e) => {
                    context.set_ui_state(UiState::Normal);
                    context.set_input(String::default());
                    context.set_status_message(&format!("Rename failed: {}", e));
                }
            }
        }
        KeyCode::Esc => {
            context.set_ui_state(UiState::Normal);
            context.set_input(String::default());
        }
        KeyCode::Char(c) => {
            context.input.push(c);
        }
        _ => {}
    }
    Ok(())
}

/// Handles user input events for the confirmation popup
pub fn handle_confirm_popup_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    match key_event.code {
        KeyCode::Char('y') => {
            context.set_ui_state(UiState::Normal);
            context.set_confirm_button_selected();
            spawn_delete_operation(context);
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            context.set_ui_state(UiState::Normal);
        }
        KeyCode::Left => {
            if !context.get_confirm_button_selected().unwrap_or(false) {
                context.set_confirm_button_selected()
            }
        }
        KeyCode::Right => {
            if context.get_confirm_button_selected().unwrap_or(false) {
                context.set_confirm_button_selected()
            }
        }
        KeyCode::Enter => {
            context.set_ui_state(UiState::Normal);

            if context.get_confirm_button_selected().unwrap_or(false) {
                spawn_delete_operation(context);
                context.set_confirm_button_selected()
            }
        }
        _ => {}
    }
    Ok(())
}

fn spawn_delete_operation(context: &mut Context) {
    if context.is_operation_running() {
        context.set_status_message("Operation already in progress");
        return;
    }

    let panel = context.active();
    let selected = match panel.get_selected_item() {
        Some(item) => item.clone(),
        None => return,
    };

    let path = std::path::PathBuf::from(&panel.path).join(&selected);
    let desc = format!("Deleting {}...", selected);
    let rx = background_op::spawn_delete(path, desc.clone());
    context.start_operation(rx, desc);
}

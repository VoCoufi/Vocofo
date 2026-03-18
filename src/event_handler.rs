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
            // If filter is active, clear it instead of quitting
            if !context.active().filter.is_empty() {
                context.active_mut().clear_filter();
                return Ok(());
            }
            context.set_exit();
        }
        (KeyCode::Enter, _) => {
            if let Some(err) = context.active_mut().open_item() {
                context.set_status_message(&err);
            }
            context.active_mut().clear_filter();
            context.active_mut().invalidate_directory_cache();
        }
        (KeyCode::Tab, _) => {
            context.toggle_active_panel();
        }
        (KeyCode::Backspace, _) => {
            if let Some(err) = context.active_mut().navigate_to_parent() {
                context.set_status_message(&err);
            }
            context.active_mut().clear_filter();
        }
        (KeyCode::Down, _) => {
            let panel = context.active_mut();
            if panel.filtered_items.len() > panel.state + 1 {
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
            handle_paste(context);
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
        (KeyCode::Char('='), _) => {
            let path = context.active().path.clone();
            let other = 1 - context.active_panel;
            context.panels[other].path = path;
            context.panels[other].invalidate_directory_cache();
            context.panels[other].clear_filter();
            context.set_status_message("Panels synced");
        }
        (KeyCode::Char('/'), _) => {
            context.set_ui_state(UiState::SearchMode);
        }
        (KeyCode::F(3), _) => {
            context.show_preview = !context.show_preview;
        }
        _ => {}
    }

    if context.show_preview {
        context.active_mut().update_preview();
    }

    Ok(())
}

/// Handles key events for the search/filter mode
pub fn handle_search_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    match key_event.code {
        KeyCode::Backspace => {
            let panel = context.active_mut();
            if !panel.filter.is_empty() {
                panel.filter.pop();
                panel.apply_filter();
            }
        }
        KeyCode::Enter => {
            // Focus on the selected item in the full list, then clear filter
            let panel = context.active_mut();
            let selected_name = panel.get_selected_item().cloned();
            panel.clear_filter();
            // Restore cursor to the same item in the unfiltered list
            if let Some(name) = selected_name {
                if let Some(pos) = panel.filtered_items.iter().position(|i| i == &name) {
                    panel.state = pos;
                }
            }
            context.set_ui_state(UiState::Normal);
        }
        KeyCode::Esc => {
            context.active_mut().clear_filter();
            context.set_ui_state(UiState::Normal);
        }
        KeyCode::Down => {
            let panel = context.active_mut();
            if panel.filtered_items.len() > panel.state + 1 {
                panel.increment_state();
            }
        }
        KeyCode::Up => {
            let panel = context.active_mut();
            if panel.state > 0 {
                panel.decrease_state();
            }
        }
        KeyCode::Char(c) => {
            let panel = context.active_mut();
            panel.filter.push(c);
            panel.apply_filter();
        }
        _ => {}
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

/// Handles user input events for the delete confirmation popup
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

/// Handles user input events for the overwrite confirmation popup
pub fn handle_overwrite_popup_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    match key_event.code {
        KeyCode::Char('y') => {
            context.set_ui_state(UiState::Normal);
            execute_pending_paste(context);
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            context.set_ui_state(UiState::Normal);
            context.pending_paste = None;
            context.set_status_message("Paste cancelled");
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
                execute_pending_paste(context);
            } else {
                context.pending_paste = None;
                context.set_status_message("Paste cancelled");
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_paste(context: &mut Context) {
    if context.is_operation_running() {
        context.set_status_message("Operation already in progress");
        return;
    }
    if context.get_copy_path().is_empty() {
        context.set_status_message("Nothing to paste — copy a file first");
        return;
    }
    match file_operation::resolve_paste_paths(context) {
        Ok((from, to)) => {
            let is_cut = context.clipboard_mode == crate::context::ClipboardMode::Cut;
            if to.exists() {
                // Destination exists — ask for confirmation
                context.pending_paste = Some((from, to, is_cut));
                context.confirm_popup_size = true; // Default to Yes
                context.set_ui_state(UiState::ConfirmOverwrite);
            } else {
                spawn_paste_operation(context, from, to, is_cut);
            }
        }
        Err(e) => {
            context.set_status_message(&format!("Paste failed: {}", e));
        }
    }
}

fn execute_pending_paste(context: &mut Context) {
    if let Some((from, to, is_cut)) = context.pending_paste.take() {
        // Delete existing target before overwriting
        if to.exists() {
            if let Err(e) = file_operation::delete(&to) {
                context.set_status_message(&format!("Cannot remove existing file: {}", e));
                return;
            }
        }
        spawn_paste_operation(context, from, to, is_cut);
    }
}

fn spawn_paste_operation(
    context: &mut Context,
    from: std::path::PathBuf,
    to: std::path::PathBuf,
    is_cut: bool,
) {
    let name = from.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "item".to_string());

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

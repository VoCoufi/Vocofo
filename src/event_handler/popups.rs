use crate::context::{Context, UiState};
use crate::file_operation;
use crossterm::event::{KeyCode, KeyEvent};

use super::EventResult;
use super::clipboard::{execute_pending_paste, spawn_delete_operation};

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
            let panel = context.active_mut();
            let selected_name = panel.get_selected_item().cloned();
            panel.clear_filter();
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

// ============================================================================
// Input popup handlers — shared pattern with per-popup Enter action
// ============================================================================

/// Generic input popup handler: Backspace, Esc, Char, with custom Enter action
fn handle_input_popup(
    context: &mut Context,
    key_event: KeyEvent,
    on_enter: fn(&mut Context) -> file_operation::FileResult<()>,
    success_msg: &str,
    error_prefix: &str,
) -> EventResult {
    match key_event.code {
        KeyCode::Backspace => {
            if !context.input.is_empty() {
                context.input.pop();
            }
        }
        KeyCode::Enter => match on_enter(context) {
            Ok(_) => {
                context.active_mut().invalidate_directory_cache();
                context.set_status_message(success_msg);
            }
            Err(e) => {
                context.set_ui_state(UiState::Normal);
                context.set_input(String::default());
                context.set_status_message(&format!("{}: {}", error_prefix, e));
            }
        },
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

pub fn handle_popup_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    handle_input_popup(
        context,
        key_event,
        file_operation::handle_create_directory,
        "Folder created",
        "Create failed",
    )
}

pub fn handle_file_popup_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    handle_input_popup(
        context,
        key_event,
        file_operation::handle_create_file,
        "File created",
        "Create failed",
    )
}

pub fn handle_rename_popup_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    handle_input_popup(
        context,
        key_event,
        file_operation::handle_rename,
        "Renamed successfully",
        "Rename failed",
    )
}

/// Chmod has custom input filtering (octal only) so it stays separate
pub fn handle_chmod_popup_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    match key_event.code {
        KeyCode::Backspace => {
            if !context.input.is_empty() {
                context.input.pop();
            }
        }
        KeyCode::Enter => {
            let input = context.input.clone();
            let mode = u32::from_str_radix(&input, 8);
            match mode {
                Ok(m) if m <= 0o7777 => {
                    if let Some(item) = context.active().get_selected_item() {
                        let name = item.trim_end_matches('/').to_string();
                        let path = context
                            .active()
                            .backend
                            .join_path(&context.active().path, &name);
                        match context.active().backend.chmod(&path, m) {
                            Ok(()) => {
                                context.active_mut().invalidate_directory_cache();
                                context
                                    .set_status_message(&format!("Permissions changed to {:o}", m));
                            }
                            Err(e) => {
                                context.set_status_message(&format!("chmod failed: {}", e));
                            }
                        }
                    }
                    context.set_ui_state(UiState::Normal);
                    context.set_input(String::default());
                }
                _ => {
                    context.set_status_message("Invalid octal mode (use e.g. 755)");
                    context.set_ui_state(UiState::Normal);
                    context.set_input(String::default());
                }
            }
        }
        KeyCode::Esc => {
            context.set_ui_state(UiState::Normal);
            context.set_input(String::default());
        }
        KeyCode::Char(c) if c.is_ascii_digit() && c < '8' => {
            if context.input.len() < 4 {
                context.input.push(c);
            }
        }
        _ => {}
    }
    Ok(())
}

// ============================================================================
// Confirm dialog handlers — shared button navigation pattern
// ============================================================================

/// Generic confirm dialog handler with Yes/No button navigation
fn handle_confirm_dialog(
    context: &mut Context,
    key_event: KeyEvent,
    on_confirm: fn(&mut Context),
    on_cancel: fn(&mut Context),
) -> EventResult {
    match key_event.code {
        KeyCode::Char('y') => {
            context.set_ui_state(UiState::Normal);
            context.set_confirm_button_selected();
            on_confirm(context);
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            context.set_ui_state(UiState::Normal);
            on_cancel(context);
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
                on_confirm(context);
                context.set_confirm_button_selected()
            } else {
                on_cancel(context);
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn handle_confirm_popup_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    handle_confirm_dialog(context, key_event, spawn_delete_operation, |_| {})
}

pub fn handle_overwrite_popup_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    handle_confirm_dialog(context, key_event, execute_pending_paste, |ctx| {
        ctx.pending_paste = None;
        ctx.set_status_message("Paste cancelled");
    })
}

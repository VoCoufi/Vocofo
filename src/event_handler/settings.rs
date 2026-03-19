use crate::context::{Context, UiState};
use crossterm::event::{KeyCode, KeyEvent};

use super::EventResult;

/// Handles key events for the settings popup
pub fn handle_settings_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    let state = match context.settings_state.as_mut() {
        Some(s) => s,
        None => {
            context.set_ui_state(UiState::Normal);
            return Ok(());
        }
    };

    if state.editing_path {
        match key_event.code {
            KeyCode::Enter => {
                context.config.general.default_path = state.path_input.clone();
                state.editing_path = false;
            }
            KeyCode::Esc => {
                state.path_input = context.config.general.default_path.clone();
                state.editing_path = false;
            }
            KeyCode::Backspace => {
                state.path_input.pop();
            }
            KeyCode::Char(c) => {
                state.path_input.push(c);
            }
            _ => {}
        }
        return Ok(());
    }

    match key_event.code {
        KeyCode::Esc => {
            let _ = context.config.save();
            context.settings_state = None;
            context.set_ui_state(UiState::Normal);
            context.set_status_message("Settings saved");
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if state.focused_field > 0 {
                state.focused_field -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if state.focused_field + 1 < state.field_count() {
                state.focused_field += 1;
            }
        }
        KeyCode::Left | KeyCode::Right => {
            if state.focused_field == 0 {
                context.config.general.panel_layout = if key_event.code == KeyCode::Right {
                    context.config.general.panel_layout.next()
                } else {
                    context.config.general.panel_layout.prev()
                };
            }
        }
        KeyCode::Char(' ') | KeyCode::Enter => match state.focused_field {
            0 => {
                context.config.general.panel_layout = context.config.general.panel_layout.next();
            }
            1 => {
                let new_val = !context.config.general.show_hidden;
                context.config.general.show_hidden = new_val;
                context.panels[0].show_hidden = new_val;
                context.panels[1].show_hidden = new_val;
                context.panels[0].invalidate_directory_cache();
                context.panels[1].invalidate_directory_cache();
            }
            2 => {
                context.config.general.show_preview_on_start =
                    !context.config.general.show_preview_on_start;
            }
            3 => {
                if key_event.code == KeyCode::Enter {
                    state.editing_path = true;
                }
            }
            _ => {}
        },
        _ => {}
    }
    Ok(())
}

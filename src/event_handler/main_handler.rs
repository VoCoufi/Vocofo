use std::sync::Arc;

use crate::context::{CommandPaletteState, ConnectDialogState, Context, SettingsState, UiState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::EventResult;
use super::clipboard::{handle_copy_or_cut, handle_paste};
use super::command_palette::PALETTE_ACTIONS;

/// Handles the main event loop for the application
pub fn handle_main_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    context.clear_status_message();

    // Handle pending gg sequence
    if context.pending_g {
        context.pending_g = false;
        if key_event.code == KeyCode::Char('g') {
            context.active_mut().go_to_first();
            return Ok(());
        }
    }

    match (key_event.code, key_event.modifiers) {
        (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => {
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
        (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
            context.set_ui_state(UiState::CreatePopup);
        }
        (KeyCode::Char(' '), _) => {
            context.active_mut().toggle_selection();
        }
        (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
            context.active_mut().select_all();
            let count = context.active().selected.len();
            context.set_status_message(&format!("{} items selected", count));
        }
        (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
            context.active_mut().clear_selection();
            context.set_status_message("Selection cleared");
        }
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            handle_copy_or_cut(context, crate::context::ClipboardMode::Copy);
        }
        (KeyCode::Char('v'), KeyModifiers::CONTROL) => {
            handle_paste(context);
        }
        (KeyCode::Char('x'), KeyModifiers::CONTROL) => {
            handle_copy_or_cut(context, crate::context::ClipboardMode::Cut);
        }
        (KeyCode::Delete, _) => {
            context.set_ui_state(UiState::ConfirmDelete);
        }
        (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
            if let Some(item) = context.active().get_selected_item() {
                if item != "../" {
                    let name = item.trim_end_matches('/').to_string();
                    context.set_input(name);
                    context.set_ui_state(UiState::RenamePopup);
                }
            }
        }
        (KeyCode::Char('m'), KeyModifiers::CONTROL) => {
            if let Some(item) = context.active().get_selected_item() {
                if item != "../" {
                    let name = item.trim_end_matches('/').to_string();
                    let path = context
                        .active()
                        .backend
                        .join_path(&context.active().path, &name);
                    match context.active().backend.metadata(&path) {
                        Ok(info) => {
                            let current_mode = info
                                .mode
                                .map(|m| format!("{:o}", m & 0o7777))
                                .unwrap_or_else(|| "644".to_string());
                            context.set_input(current_mode);
                            context.set_ui_state(UiState::ChmodPopup);
                        }
                        Err(e) => {
                            context.set_status_message(&format!("Cannot read permissions: {}", e));
                        }
                    }
                }
            }
        }
        (KeyCode::Char('='), _) => {
            let path = context.active().path.clone();
            let backend = Arc::clone(&context.active().backend);
            let other = 1 - context.active_panel;
            context.panels[other].path = path;
            context.panels[other].backend = backend;
            context.panels[other].invalidate_directory_cache();
            context.panels[other].clear_filter();
            context.set_status_message("Panels synced");
        }
        (KeyCode::Char('/'), _) => {
            context.set_ui_state(UiState::SearchMode);
        }
        (KeyCode::Char('.'), _) => {
            let panel = context.active_mut();
            panel.show_hidden = !panel.show_hidden;
            panel.invalidate_directory_cache();
            let state = if panel.show_hidden { "shown" } else { "hidden" };
            context.set_status_message(&format!("Hidden files {}", state));
        }
        (KeyCode::F(1), _) => {
            context.command_palette = Some(CommandPaletteState::new(PALETTE_ACTIONS.len()));
            context.set_ui_state(UiState::CommandPalette);
        }
        (KeyCode::F(2), _) => {
            let path = context.config.general.default_path.clone();
            context.settings_state = Some(SettingsState::new(&path));
            context.set_ui_state(UiState::SettingsPopup);
        }
        (KeyCode::F(3), _) => {
            context.show_preview = !context.show_preview;
        }
        (KeyCode::F(5), _) => {
            context.connect_dialog = Some(ConnectDialogState::new());
            context.set_ui_state(UiState::ConnectDialog);
        }
        (KeyCode::F(6), _) => {
            if !context.active().backend.is_local() {
                context.active().backend.disconnect();
                let local_backend: Arc<dyn crate::backend::FilesystemBackend> =
                    Arc::new(crate::local_backend::LocalBackend::new());
                let home = local_backend
                    .canonicalize(".")
                    .unwrap_or_else(|_| ".".to_string());
                context.active_mut().backend = local_backend;
                context.active_mut().path = home;
                context.active_mut().invalidate_directory_cache();
                context.set_status_message("Disconnected");
            }
        }
        (KeyCode::F(7), _) => {
            if context.config.connections.is_empty() {
                context.set_status_message("No saved bookmarks");
            } else {
                context.bookmark_selected = 0;
                context.set_ui_state(UiState::BookmarkList);
            }
        }
        (KeyCode::PageDown, _) => {
            context.active_mut().page_down();
        }
        (KeyCode::PageUp, _) => {
            context.active_mut().page_up();
        }
        (KeyCode::Home, _) => {
            context.active_mut().go_to_first();
        }
        (KeyCode::End, _) => {
            context.active_mut().go_to_last();
        }
        // Vim-style navigation
        (KeyCode::Char('j'), _) => {
            let panel = context.active_mut();
            if panel.filtered_items.len() > panel.state + 1 {
                panel.increment_state();
            }
        }
        (KeyCode::Char('k'), _) => {
            let panel = context.active_mut();
            if panel.state > 0 {
                panel.decrease_state();
            }
        }
        (KeyCode::Char('l'), _) => {
            if let Some(err) = context.active_mut().open_item() {
                context.set_status_message(&err);
            }
            context.active_mut().clear_filter();
            context.active_mut().invalidate_directory_cache();
        }
        (KeyCode::Char('h'), _) => {
            if let Some(err) = context.active_mut().navigate_to_parent() {
                context.set_status_message(&err);
            }
            context.active_mut().clear_filter();
        }
        (KeyCode::Char('G'), _) => {
            context.active_mut().go_to_last();
        }
        (KeyCode::Char('g'), _) => {
            context.pending_g = true;
        }
        (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
            context.set_ui_state(UiState::CreateFilePopup);
        }
        _ => {}
    }

    if context.show_preview {
        context.active_mut().update_preview();
    }

    Ok(())
}

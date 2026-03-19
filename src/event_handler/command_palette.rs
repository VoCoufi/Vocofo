use std::sync::Arc;

use crate::context::{CommandPaletteState, ConnectDialogState, Context, SettingsState, UiState};
use crossterm::event::{KeyCode, KeyEvent};

use super::EventResult;
use super::clipboard::{handle_copy_or_cut, handle_paste};

/// Command palette action definition
pub struct PaletteAction {
    pub label: &'static str,
    pub shortcut: &'static str,
    pub id: &'static str,
    pub section: &'static str,
}

/// All available actions for the command palette
pub const PALETTE_ACTIONS: &[PaletteAction] = &[
    // File Operations
    PaletteAction { id: "delete", label: "Delete", shortcut: "Del", section: "File Operations" },
    PaletteAction { id: "rename", label: "Rename", shortcut: "^R", section: "File Operations" },
    PaletteAction { id: "copy", label: "Copy", shortcut: "^C", section: "File Operations" },
    PaletteAction { id: "paste", label: "Paste", shortcut: "^V", section: "File Operations" },
    PaletteAction { id: "cut", label: "Cut", shortcut: "^X", section: "File Operations" },
    PaletteAction { id: "chmod", label: "Change Permissions", shortcut: "^M", section: "File Operations" },
    // Create
    PaletteAction { id: "new_file", label: "New File", shortcut: "^N", section: "Create" },
    PaletteAction { id: "new_folder", label: "New Folder", shortcut: "^P", section: "Create" },
    // Remote
    PaletteAction { id: "connect", label: "Connect (SFTP/FTP)", shortcut: "F5", section: "Remote" },
    PaletteAction { id: "disconnect", label: "Disconnect", shortcut: "F6", section: "Remote" },
    PaletteAction { id: "bookmarks", label: "Bookmarks", shortcut: "F7", section: "Remote" },
    // View
    PaletteAction { id: "toggle_hidden", label: "Toggle Hidden Files", shortcut: ".", section: "View" },
    PaletteAction { id: "toggle_preview", label: "Toggle Preview", shortcut: "F3", section: "View" },
    PaletteAction { id: "sync_panels", label: "Sync Panels", shortcut: "=", section: "View" },
    PaletteAction { id: "search", label: "Search / Filter", shortcut: "/", section: "View" },
    // Selection
    PaletteAction { id: "select_all", label: "Select All", shortcut: "^A", section: "Selection" },
    PaletteAction { id: "deselect", label: "Deselect All", shortcut: "^D", section: "Selection" },
    // App
    PaletteAction { id: "settings", label: "Settings", shortcut: "F2", section: "App" },
    PaletteAction { id: "quit", label: "Quit", shortcut: "Q", section: "App" },
];

/// Handles key events for the command palette
pub fn handle_command_palette_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    let state = match context.command_palette.as_mut() {
        Some(s) => s,
        None => {
            context.set_ui_state(UiState::Normal);
            return Ok(());
        }
    };

    match key_event.code {
        KeyCode::Esc => {
            context.command_palette = None;
            context.set_ui_state(UiState::Normal);
        }
        KeyCode::Up => {
            if state.selected > 0 { state.selected -= 1; }
        }
        KeyCode::Down => {
            if !state.filtered_indices.is_empty() && state.selected + 1 < state.filtered_indices.len() {
                state.selected += 1;
            }
        }
        KeyCode::Enter => {
            if let Some(&action_idx) = state.filtered_indices.get(state.selected) {
                let action_id = PALETTE_ACTIONS[action_idx].id;
                context.command_palette = None;
                context.set_ui_state(UiState::Normal);
                execute_palette_action(context, action_id);
            }
        }
        KeyCode::Backspace => {
            if !state.filter.is_empty() {
                state.filter.pop();
                refilter_palette(state);
            }
        }
        KeyCode::Char(c) => {
            state.filter.push(c);
            refilter_palette(state);
        }
        _ => {}
    }
    Ok(())
}

fn refilter_palette(state: &mut CommandPaletteState) {
    let query = state.filter.to_lowercase();
    state.filtered_indices = (0..PALETTE_ACTIONS.len())
        .filter(|&i| {
            if query.is_empty() { return true; }
            let label = PALETTE_ACTIONS[i].label.to_lowercase();
            let id = PALETTE_ACTIONS[i].id.to_lowercase();
            label.contains(&query) || id.contains(&query)
        })
        .collect();
    if state.selected >= state.filtered_indices.len() {
        state.selected = state.filtered_indices.len().saturating_sub(1);
    }
}

fn execute_palette_action(context: &mut Context, action_id: &str) {
    match action_id {
        "delete" => { context.set_ui_state(UiState::ConfirmDelete); }
        "rename" => {
            if let Some(item) = context.active().get_selected_item() {
                if item != "../" {
                    let name = item.trim_end_matches('/').to_string();
                    context.set_input(name);
                    context.set_ui_state(UiState::RenamePopup);
                }
            }
        }
        "copy" => { handle_copy_or_cut(context, crate::context::ClipboardMode::Copy); }
        "paste" => { handle_paste(context); }
        "cut" => { handle_copy_or_cut(context, crate::context::ClipboardMode::Cut); }
        "chmod" => {
            if let Some(item) = context.active().get_selected_item() {
                if item != "../" {
                    let name = item.trim_end_matches('/').to_string();
                    let path = context.active().backend.join_path(&context.active().path, &name);
                    match context.active().backend.metadata(&path) {
                        Ok(info) => {
                            let current_mode = info.mode
                                .map(|m| format!("{:o}", m & 0o7777))
                                .unwrap_or_else(|| "644".to_string());
                            context.set_input(current_mode);
                            context.set_ui_state(UiState::ChmodPopup);
                        }
                        Err(e) => { context.set_status_message(&format!("Cannot read permissions: {}", e)); }
                    }
                }
            }
        }
        "new_file" => { context.set_ui_state(UiState::CreateFilePopup); }
        "new_folder" => { context.set_ui_state(UiState::CreatePopup); }
        "connect" => {
            context.connect_dialog = Some(ConnectDialogState::new());
            context.set_ui_state(UiState::ConnectDialog);
        }
        "disconnect" => {
            if !context.active().backend.is_local() {
                context.active().backend.disconnect();
                let local_backend: Arc<dyn crate::backend::FilesystemBackend> =
                    Arc::new(crate::local_backend::LocalBackend::new());
                let home = local_backend.canonicalize(".").unwrap_or_else(|_| ".".to_string());
                context.active_mut().backend = local_backend;
                context.active_mut().path = home;
                context.active_mut().invalidate_directory_cache();
                context.set_status_message("Disconnected");
            }
        }
        "bookmarks" => {
            if context.config.connections.is_empty() {
                context.set_status_message("No saved bookmarks");
            } else {
                context.bookmark_selected = 0;
                context.set_ui_state(UiState::BookmarkList);
            }
        }
        "settings" => {
            let path = context.config.general.default_path.clone();
            context.settings_state = Some(SettingsState::new(&path));
            context.set_ui_state(UiState::SettingsPopup);
        }
        "toggle_hidden" => {
            let panel = context.active_mut();
            panel.show_hidden = !panel.show_hidden;
            panel.invalidate_directory_cache();
            let state = if panel.show_hidden { "shown" } else { "hidden" };
            context.set_status_message(&format!("Hidden files {}", state));
        }
        "toggle_preview" => { context.show_preview = !context.show_preview; }
        "sync_panels" => {
            let path = context.active().path.clone();
            let backend = Arc::clone(&context.active().backend);
            let other = 1 - context.active_panel;
            context.panels[other].path = path;
            context.panels[other].backend = backend;
            context.panels[other].invalidate_directory_cache();
            context.panels[other].clear_filter();
            context.set_status_message("Panels synced");
        }
        "select_all" => {
            context.active_mut().select_all();
            let count = context.active().selected.len();
            context.set_status_message(&format!("{} items selected", count));
        }
        "deselect" => {
            context.active_mut().clear_selection();
            context.set_status_message("Selection cleared");
        }
        "search" => { context.set_ui_state(UiState::SearchMode); }
        "quit" => { context.set_exit(); }
        _ => {}
    }
}

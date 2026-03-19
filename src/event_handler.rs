use std::sync::Arc;

use crate::background_op;
use crate::context::{CommandPaletteState, ConnectDialogState, ConnectionProtocol, Context, SettingsState, UiState};
use crate::file_operation;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Result type for event handler operations
type EventResult = Result<(), Box<dyn std::error::Error>>;

/// Command palette action definition
pub struct PaletteAction {
    pub label: &'static str,
    pub shortcut: &'static str,
    pub id: &'static str,
}

/// All available actions for the command palette
pub const PALETTE_ACTIONS: &[PaletteAction] = &[
    PaletteAction { id: "delete", label: "Delete", shortcut: "Del" },
    PaletteAction { id: "rename", label: "Rename", shortcut: "^R" },
    PaletteAction { id: "copy", label: "Copy", shortcut: "^C" },
    PaletteAction { id: "paste", label: "Paste", shortcut: "^V" },
    PaletteAction { id: "cut", label: "Cut", shortcut: "^X" },
    PaletteAction { id: "chmod", label: "Change Permissions", shortcut: "^M" },
    PaletteAction { id: "new_file", label: "New File", shortcut: "^N" },
    PaletteAction { id: "new_folder", label: "New Folder", shortcut: "^P" },
    PaletteAction { id: "connect", label: "Connect (SFTP/FTP)", shortcut: "F5" },
    PaletteAction { id: "disconnect", label: "Disconnect", shortcut: "F6" },
    PaletteAction { id: "bookmarks", label: "Bookmarks", shortcut: "F7" },
    PaletteAction { id: "settings", label: "Settings", shortcut: "F2" },
    PaletteAction { id: "toggle_hidden", label: "Toggle Hidden Files", shortcut: "." },
    PaletteAction { id: "toggle_preview", label: "Toggle Preview", shortcut: "F3" },
    PaletteAction { id: "sync_panels", label: "Sync Panels", shortcut: "=" },
    PaletteAction { id: "select_all", label: "Select All", shortcut: "^A" },
    PaletteAction { id: "deselect", label: "Deselect All", shortcut: "^D" },
    PaletteAction { id: "search", label: "Search / Filter", shortcut: "/" },
    PaletteAction { id: "quit", label: "Quit", shortcut: "Q" },
];

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
        // Not 'g' — fall through to normal handling
    }

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
                    let path = context.active().backend.join_path(&context.active().path, &name);
                    match context.active().backend.metadata(&path) {
                        Ok(info) => {
                            let current_mode = info.mode
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
            // Disconnect: explicitly close connection, then reset to local backend
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
        (KeyCode::F(7), _) => {
            if context.config.connections.is_empty() {
                context.set_status_message("No saved bookmarks");
            } else {
                context.bookmark_selected = 0;
                context.set_ui_state(UiState::BookmarkList);
            }
        }
        // Page navigation
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
        // Create file
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

/// Handles key events for the create file popup
pub fn handle_file_popup_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    match key_event.code {
        KeyCode::Backspace => {
            if !context.input.is_empty() {
                context.input.pop();
            }
        }
        KeyCode::Enter => {
            match file_operation::handle_create_file(context) {
                Ok(_) => {
                    context.active_mut().invalidate_directory_cache();
                    context.set_status_message("File created");
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
            context.set_input(String::default());
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

/// Handles key events for the chmod popup
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
                        let path = context.active().backend.join_path(&context.active().path, &name);
                        match context.active().backend.chmod(&path, m) {
                            Ok(()) => {
                                context.active_mut().invalidate_directory_cache();
                                context.set_status_message(&format!("Permissions changed to {:o}", m));
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

/// Handles key events for the connection dialog
pub fn handle_connect_dialog_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    let dialog = match context.connect_dialog.as_mut() {
        Some(d) => d,
        None => {
            context.set_ui_state(UiState::Normal);
            return Ok(());
        }
    };

    match key_event.code {
        KeyCode::Esc => {
            context.connect_dialog = None;
            context.set_ui_state(UiState::Normal);
        }
        KeyCode::Tab => {
            dialog.focused_field = (dialog.focused_field + 1) % dialog.field_count();
        }
        KeyCode::BackTab => {
            if dialog.focused_field == 0 {
                dialog.focused_field = dialog.field_count() - 1;
            } else {
                dialog.focused_field -= 1;
            }
        }
        KeyCode::Up if dialog.focused_field == 0 => {
            // Toggle protocol
            dialog.protocol = match dialog.protocol {
                ConnectionProtocol::Sftp => ConnectionProtocol::Ftp,
                ConnectionProtocol::Ftp => ConnectionProtocol::Sftp,
            };
            // Update default port
            dialog.port = match dialog.protocol {
                ConnectionProtocol::Sftp => "22".to_string(),
                ConnectionProtocol::Ftp => "21".to_string(),
            };
        }
        KeyCode::Down if dialog.focused_field == 0 => {
            dialog.protocol = match dialog.protocol {
                ConnectionProtocol::Sftp => ConnectionProtocol::Ftp,
                ConnectionProtocol::Ftp => ConnectionProtocol::Sftp,
            };
            dialog.port = match dialog.protocol {
                ConnectionProtocol::Sftp => "22".to_string(),
                ConnectionProtocol::Ftp => "21".to_string(),
            };
        }
        KeyCode::Backspace if dialog.focused_field > 0 => {
            dialog.active_field_mut().pop();
        }
        KeyCode::Char('s') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            // Save as bookmark — switch to name input
            if dialog.host.is_empty() {
                dialog.error_message = Some("Host is required to save bookmark".to_string());
            } else {
                let default_name = format!("{}@{}", dialog.username, dialog.host);
                context.input = default_name;
                context.set_ui_state(UiState::BookmarkNameInput);
            }
        }
        KeyCode::Char(c) if dialog.focused_field > 0 => {
            dialog.active_field_mut().push(c);
        }
        KeyCode::Enter => {
            attempt_connection(context);
        }
        _ => {}
    }
    Ok(())
}

/// Handles key events for the bookmark list popup
pub fn handle_bookmark_list_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    let count = context.config.connections.len();
    match key_event.code {
        KeyCode::Esc => {
            context.set_ui_state(UiState::Normal);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if context.bookmark_selected > 0 {
                context.bookmark_selected -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if context.bookmark_selected + 1 < count {
                context.bookmark_selected += 1;
            }
        }
        KeyCode::Char('d') => {
            if context.bookmark_selected < count {
                context.config.connections.remove(context.bookmark_selected);
                if context.bookmark_selected >= context.config.connections.len() && context.bookmark_selected > 0 {
                    context.bookmark_selected -= 1;
                }
                let _ = context.config.save();
                if context.config.connections.is_empty() {
                    context.set_ui_state(UiState::Normal);
                    context.set_status_message("Bookmark deleted");
                }
            }
        }
        KeyCode::Enter => {
            if context.bookmark_selected < count {
                let profile = &context.config.connections[context.bookmark_selected];
                let mut dialog = ConnectDialogState::new();
                dialog.protocol = match profile.protocol.as_str() {
                    "ftp" => ConnectionProtocol::Ftp,
                    _ => ConnectionProtocol::Sftp,
                };
                dialog.host = profile.host.clone();
                dialog.port = profile.port.to_string();
                dialog.username = profile.username.clone();
                dialog.key_path = profile.key_path.clone().unwrap_or_default();
                // password intentionally left empty
                context.connect_dialog = Some(dialog);
                context.set_ui_state(UiState::ConnectDialog);
            }
        }
        _ => {}
    }
    Ok(())
}

/// Handles key events for the bookmark name input popup
pub fn handle_bookmark_name_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    match key_event.code {
        KeyCode::Esc => {
            context.set_input(String::default());
            context.set_ui_state(UiState::ConnectDialog);
        }
        KeyCode::Backspace => {
            context.input.pop();
        }
        KeyCode::Enter => {
            let name = context.input.clone();
            if name.is_empty() {
                context.set_ui_state(UiState::ConnectDialog);
                return Ok(());
            }
            if let Some(dialog) = &context.connect_dialog {
                let protocol = match dialog.protocol {
                    ConnectionProtocol::Sftp => "sftp",
                    ConnectionProtocol::Ftp => "ftp",
                };
                let port: u16 = dialog.port.parse().unwrap_or(match dialog.protocol {
                    ConnectionProtocol::Sftp => 22,
                    ConnectionProtocol::Ftp => 21,
                });
                let key_path = if dialog.key_path.is_empty() {
                    None
                } else {
                    Some(dialog.key_path.clone())
                };
                let profile = crate::config::ConnectionProfile {
                    name: name.clone(),
                    protocol: protocol.to_string(),
                    host: dialog.host.clone(),
                    port,
                    username: dialog.username.clone(),
                    key_path,
                };
                context.config.connections.push(profile);
                let _ = context.config.save();
                context.set_status_message(&format!("Bookmark '{}' saved", name));
            }
            context.set_input(String::default());
            context.set_ui_state(UiState::ConnectDialog);
        }
        KeyCode::Char(c) => {
            context.input.push(c);
        }
        _ => {}
    }
    Ok(())
}

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
            if state.selected > 0 {
                state.selected -= 1;
            }
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
            if query.is_empty() {
                return true;
            }
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
        "delete" => {
            context.set_ui_state(UiState::ConfirmDelete);
        }
        "rename" => {
            if let Some(item) = context.active().get_selected_item() {
                if item != "../" {
                    let name = item.trim_end_matches('/').to_string();
                    context.set_input(name);
                    context.set_ui_state(UiState::RenamePopup);
                }
            }
        }
        "copy" => {
            handle_copy_or_cut(context, crate::context::ClipboardMode::Copy);
        }
        "paste" => {
            handle_paste(context);
        }
        "cut" => {
            handle_copy_or_cut(context, crate::context::ClipboardMode::Cut);
        }
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
                        Err(e) => {
                            context.set_status_message(&format!("Cannot read permissions: {}", e));
                        }
                    }
                }
            }
        }
        "new_file" => {
            context.set_ui_state(UiState::CreateFilePopup);
        }
        "new_folder" => {
            context.set_ui_state(UiState::CreatePopup);
        }
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
        "toggle_preview" => {
            context.show_preview = !context.show_preview;
        }
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
        "search" => {
            context.set_ui_state(UiState::SearchMode);
        }
        "quit" => {
            context.set_exit();
        }
        _ => {}
    }
}

/// Handles key events for the settings popup
pub fn handle_settings_event(context: &mut Context, key_event: KeyEvent) -> EventResult {
    let state = match context.settings_state.as_mut() {
        Some(s) => s,
        None => {
            context.set_ui_state(UiState::Normal);
            return Ok(());
        }
    };

    // If editing path, handle typing
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
            // Save and close
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
            match state.focused_field {
                0 => {
                    // Cycle panel layout
                    context.config.general.panel_layout = if key_event.code == KeyCode::Right {
                        context.config.general.panel_layout.next()
                    } else {
                        context.config.general.panel_layout.prev()
                    };
                }
                _ => {}
            }
        }
        KeyCode::Char(' ') | KeyCode::Enter => {
            match state.focused_field {
                0 => {
                    context.config.general.panel_layout = context.config.general.panel_layout.next();
                }
                1 => {
                    // Toggle hidden files
                    let new_val = !context.config.general.show_hidden;
                    context.config.general.show_hidden = new_val;
                    context.panels[0].show_hidden = new_val;
                    context.panels[1].show_hidden = new_val;
                    context.panels[0].invalidate_directory_cache();
                    context.panels[1].invalidate_directory_cache();
                }
                2 => {
                    // Toggle preview on start
                    context.config.general.show_preview_on_start = !context.config.general.show_preview_on_start;
                }
                3 => {
                    // Start editing path
                    if key_event.code == KeyCode::Enter {
                        state.editing_path = true;
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(())
}

fn attempt_connection(context: &mut Context) {
    let dialog = match context.connect_dialog.as_ref() {
        Some(d) => d.clone(),
        None => return,
    };

    if dialog.host.is_empty() {
        if let Some(d) = context.connect_dialog.as_mut() {
            d.error_message = Some("Host is required".to_string());
        }
        return;
    }

    let port: u16 = match dialog.port.parse::<u16>() {
        Ok(0) | Err(_) => {
            if let Some(d) = context.connect_dialog.as_mut() {
                d.error_message = Some("Invalid port number".to_string());
            }
            return;
        }
        Ok(p) => p,
    };

    let result: Result<Arc<dyn crate::backend::FilesystemBackend>, String> = match dialog.protocol {
        ConnectionProtocol::Sftp => {
            #[cfg(feature = "sftp")]
            {
                let key = if dialog.key_path.is_empty() { None } else { Some(dialog.key_path.as_str()) };
                match crate::sftp_backend::SftpBackend::connect(
                    &dialog.host, port, &dialog.username, &dialog.password, key,
                ) {
                    Ok(b) => Ok(Arc::new(b) as Arc<dyn crate::backend::FilesystemBackend>),
                    Err(sftp_err) => {
                        // SFTP failed — try SCP fallback
                        match connect_scp_fallback(&dialog.host, port, &dialog.username, &dialog.password, key) {
                            Ok(b) => Ok(b),
                            Err(_) => Err(sftp_err.to_string()), // report original SFTP error
                        }
                    }
                }
            }
            #[cfg(not(feature = "sftp"))]
            {
                Err("SFTP support not compiled (enable 'sftp' feature)".to_string())
            }
        }
        ConnectionProtocol::Ftp => {
            #[cfg(feature = "ftp")]
            {
                crate::ftp_backend::FtpBackend::connect(
                    &dialog.host,
                    port,
                    &dialog.username,
                    &dialog.password,
                )
                .map(|b| Arc::new(b) as Arc<dyn crate::backend::FilesystemBackend>)
                .map_err(|e| e.to_string())
            }
            #[cfg(not(feature = "ftp"))]
            {
                Err("FTP support not compiled (enable 'ftp' feature)".to_string())
            }
        }
    };

    match result {
        Ok(backend) => {
            let initial_path = backend.canonicalize(".")
                .unwrap_or_else(|_| "/".to_string());
            let is_scp = backend.display_name().starts_with("SCP");
            context.active_mut().backend = backend;
            context.active_mut().path = initial_path;
            context.active_mut().invalidate_directory_cache();
            context.connect_dialog = None;
            context.set_ui_state(UiState::Normal);
            let mode = if is_scp { "Connected (SCP mode)" } else { "Connected" };
            context.set_status_message(mode);
        }
        Err(e) => {
            if let Some(d) = context.connect_dialog.as_mut() {
                d.error_message = Some(e);
            }
        }
    }
}

/// Try to connect via SCP fallback (SSH exec + SCP transfers)
#[cfg(feature = "sftp")]
fn connect_scp_fallback(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    key_path: Option<&str>,
) -> Result<Arc<dyn crate::backend::FilesystemBackend>, String> {
    use std::net::TcpStream;
    use ssh2::Session;

    let tcp = TcpStream::connect((host, port))
        .map_err(|e| e.to_string())?;
    let mut session = Session::new()
        .map_err(|e| e.to_string())?;
    session.set_tcp_stream(tcp);
    session.handshake()
        .map_err(|e| e.to_string())?;
    session.set_timeout(30_000);

    // Try auth methods in order: key, password, ssh-agent
    if let Some(key) = key_path {
        let passphrase = if password.is_empty() { None } else { Some(password) };
        let _ = session.userauth_pubkey_file(username, None, std::path::Path::new(key), passphrase);
    }
    if !session.authenticated() && !password.is_empty() {
        let _ = session.userauth_password(username, password);
    }
    if !session.authenticated() {
        let _ = session.userauth_agent(username);
    }
    if !session.authenticated() {
        return Err("Authentication failed".to_string());
    }

    let params = crate::backend::ConnectionParams {
        protocol: ConnectionProtocol::Sftp,
        host: host.to_string(),
        port,
        username: username.to_string(),
        password: password.to_string(),
        key_path: key_path.map(|s| s.to_string()),
    };

    Ok(Arc::new(crate::scp_backend::ScpBackend::from_session(session, params))
        as Arc<dyn crate::backend::FilesystemBackend>)
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

fn handle_copy_or_cut(context: &mut Context, mode: crate::context::ClipboardMode) {
    let label = if mode == crate::context::ClipboardMode::Cut { "Cut" } else { "Copied" };

    // Store the source backend for cross-backend transfers
    context.copy_source_backend = Some(Arc::clone(&context.active().backend));

    if context.active().has_selection() {
        context.copy_paths = context.active().get_selected_paths();
        context.copy_path = String::default();
        context.clipboard_mode = mode;
        context.set_status_message(&format!("{} {} items to clipboard", label, context.copy_paths.len()));
    } else {
        context.set_copy_path();
        context.copy_paths.clear();
        context.clipboard_mode = mode;
        context.set_status_message(&format!("{} to clipboard", label));
    }
}

fn handle_paste(context: &mut Context) {
    if context.is_operation_running() {
        context.set_status_message("Operation already in progress");
        return;
    }

    let is_cut = context.clipboard_mode == crate::context::ClipboardMode::Cut;
    let src_backend = context.copy_source_backend.clone()
        .unwrap_or_else(|| Arc::clone(&context.active().backend));
    let dst_backend = Arc::clone(&context.active().backend);

    // Multi-file paste
    if !context.copy_paths.is_empty() {
        let dest_dir = match resolve_paste_dest_dir(context) {
            Some(d) => d,
            None => return,
        };
        let items: Vec<(String, String)> = context.copy_paths.iter()
            .map(|from| {
                let name = src_backend.file_name(from).unwrap_or_default();
                let to = dst_backend.join_path(&dest_dir, &name);
                (from.clone(), to)
            })
            .collect();
        let count = items.len();
        let action = if is_cut { "Moving" } else { "Copying" };
        let desc = format!("{} {} items...", action, count);
        let progress = Arc::new(crate::background_op::TransferProgress::new());
        context.transfer_progress = Some(Arc::clone(&progress));
        let rx = background_op::spawn_copy_batch_with_backend(
            src_backend, dst_backend, items, desc.clone(), is_cut, Some(progress),
        );
        context.start_operation(rx, desc);
        return;
    }

    // Single file paste
    if context.get_copy_path().is_empty() {
        context.set_status_message("Nothing to paste — copy a file first");
        return;
    }
    match file_operation::resolve_paste_paths(context) {
        Ok((from, to)) => {
            if context.active().backend.exists(&to).unwrap_or(false) {
                context.pending_paste = Some((from, to, is_cut));
                context.confirm_popup_size = true;
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

fn resolve_paste_dest_dir(context: &mut Context) -> Option<String> {
    let panel = context.active();
    let base = panel.path.clone();
    if let Some(item) = panel.get_selected_item() {
        if panel.get_state() != 0 {
            let full = panel.backend.join_path(&base, item.trim_end_matches('/'));
            if let Ok(info) = panel.backend.metadata(&full) {
                if info.is_dir {
                    return Some(full);
                }
            }
        }
    }
    Some(base)
}

fn execute_pending_paste(context: &mut Context) {
    if let Some((from, to, is_cut)) = context.pending_paste.take() {
        // Delete existing target before pasting
        if context.active().backend.exists(&to).unwrap_or(false) {
            if let Err(e) = file_operation::delete_with_backend(&context.active().backend, &to) {
                context.set_status_message(&format!("Cannot remove existing file: {}", e));
                return;
            }
        }
        spawn_paste_operation(context, from, to, is_cut);
    }
}

fn spawn_paste_operation(
    context: &mut Context,
    from: String,
    to: String,
    is_cut: bool,
) {
    let src_backend = context.copy_source_backend.clone()
        .unwrap_or_else(|| Arc::clone(&context.active().backend));
    let dst_backend = Arc::clone(&context.active().backend);

    let name = src_backend.file_name(&from)
        .unwrap_or_else(|| "item".to_string());

    let progress = Arc::new(crate::background_op::TransferProgress::new());
    context.transfer_progress = Some(Arc::clone(&progress));
    let (desc, rx) = if is_cut {
        let desc = format!("Moving {}...", name);
        let rx = background_op::spawn_move_with_backend(
            src_backend, dst_backend, from, to, desc.clone(), Some(progress),
        );
        (desc, rx)
    } else {
        let desc = format!("Copying {}...", name);
        let rx = background_op::spawn_copy_with_backend(
            src_backend, dst_backend, from, to, desc.clone(), Some(progress),
        );
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
    let backend = Arc::clone(&panel.backend);

    // Batch delete if selection exists
    if panel.has_selection() {
        let paths = panel.get_selected_paths();
        let count = paths.len();
        let desc = format!("Deleting {} items...", count);
        let rx = background_op::spawn_delete_batch_with_backend(backend, paths, desc.clone());
        context.start_operation(rx, desc);
        context.active_mut().clear_selection();
        return;
    }

    let selected = match panel.get_selected_item() {
        Some(item) => item.clone(),
        None => return,
    };

    let path = panel.backend.join_path(&panel.path, &selected);
    let desc = format!("Deleting {}...", selected);
    let rx = background_op::spawn_delete_with_backend(backend, path, desc.clone());
    context.start_operation(rx, desc);
}

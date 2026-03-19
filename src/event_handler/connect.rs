use std::sync::Arc;

use crate::context::{ConnectDialogState, ConnectionProtocol, Context, UiState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::EventResult;

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
            dialog.protocol = match dialog.protocol {
                ConnectionProtocol::Sftp => ConnectionProtocol::Ftp,
                ConnectionProtocol::Ftp => ConnectionProtocol::Sftp,
            };
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
            if context.bookmark_selected > 0 { context.bookmark_selected -= 1; }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if context.bookmark_selected + 1 < count { context.bookmark_selected += 1; }
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
        KeyCode::Backspace => { context.input.pop(); }
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
                let key_path = if dialog.key_path.is_empty() { None } else { Some(dialog.key_path.clone()) };
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
        KeyCode::Char(c) => { context.input.push(c); }
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
                        match connect_scp_fallback(&dialog.host, port, &dialog.username, &dialog.password, key) {
                            Ok(b) => Ok(b),
                            Err(_) => Err(sftp_err.to_string()),
                        }
                    }
                }
            }
            #[cfg(not(feature = "sftp"))]
            { Err("SFTP support not compiled (enable 'sftp' feature)".to_string()) }
        }
        ConnectionProtocol::Ftp => {
            #[cfg(feature = "ftp")]
            {
                crate::ftp_backend::FtpBackend::connect(&dialog.host, port, &dialog.username, &dialog.password)
                    .map(|b| Arc::new(b) as Arc<dyn crate::backend::FilesystemBackend>)
                    .map_err(|e| e.to_string())
            }
            #[cfg(not(feature = "ftp"))]
            { Err("FTP support not compiled (enable 'ftp' feature)".to_string()) }
        }
    };

    match result {
        Ok(backend) => {
            let initial_path = backend.canonicalize(".").unwrap_or_else(|_| "/".to_string());
            let is_scp = backend.display_name().starts_with("SCP");
            context.active_mut().backend = backend;
            context.active_mut().path = initial_path;
            context.active_mut().invalidate_directory_cache();
            context.connect_dialog = None;
            context.set_ui_state(UiState::Normal);
            context.set_status_message(if is_scp { "Connected (SCP mode)" } else { "Connected" });
        }
        Err(e) => {
            if let Some(d) = context.connect_dialog.as_mut() {
                d.error_message = Some(e);
            }
        }
    }
}

#[cfg(feature = "sftp")]
fn connect_scp_fallback(
    host: &str, port: u16, username: &str, password: &str, key_path: Option<&str>,
) -> Result<Arc<dyn crate::backend::FilesystemBackend>, String> {
    use std::net::TcpStream;
    use ssh2::Session;

    let tcp = TcpStream::connect((host, port)).map_err(|e| e.to_string())?;
    let mut session = Session::new().map_err(|e| e.to_string())?;
    session.set_tcp_stream(tcp);
    session.handshake().map_err(|e| e.to_string())?;
    session.set_timeout(30_000);

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

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use vocofo::context::{ConnectDialogState, ConnectionProtocol, Context, UiState};
use vocofo::event_handler;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn key_shift_tab() -> KeyEvent {
    KeyEvent {
        code: KeyCode::BackTab,
        modifiers: KeyModifiers::SHIFT,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn create_context_with_dialog() -> Context {
    let mut ctx = Context::new().unwrap();
    ctx.connect_dialog = Some(ConnectDialogState::new());
    ctx.set_ui_state(UiState::ConnectDialog);
    ctx
}

// ==================== ConnectDialogState unit tests ====================

#[test]
fn test_dialog_state_defaults() {
    let state = ConnectDialogState::new();
    assert_eq!(state.protocol, ConnectionProtocol::Sftp);
    assert_eq!(state.host, "");
    assert_eq!(state.port, "22");
    assert_eq!(state.username, "");
    assert_eq!(state.password, "");
    assert_eq!(state.key_path, "");
    assert_eq!(state.focused_field, 1); // starts on host
    assert!(state.error_message.is_none());
}

#[test]
fn test_dialog_field_count() {
    let state = ConnectDialogState::new();
    assert_eq!(state.field_count(), 6);
}

#[test]
fn test_active_field_mut_host() {
    let mut state = ConnectDialogState::new();
    state.focused_field = 1;
    state.active_field_mut().push_str("example.com");
    assert_eq!(state.host, "example.com");
}

#[test]
fn test_active_field_mut_port() {
    let mut state = ConnectDialogState::new();
    state.focused_field = 2;
    *state.active_field_mut() = "2222".to_string();
    assert_eq!(state.port, "2222");
}

#[test]
fn test_active_field_mut_username() {
    let mut state = ConnectDialogState::new();
    state.focused_field = 3;
    state.active_field_mut().push_str("admin");
    assert_eq!(state.username, "admin");
}

#[test]
fn test_active_field_mut_password() {
    let mut state = ConnectDialogState::new();
    state.focused_field = 4;
    state.active_field_mut().push_str("secret");
    assert_eq!(state.password, "secret");
}

#[test]
fn test_active_field_mut_key_path() {
    let mut state = ConnectDialogState::new();
    state.focused_field = 5;
    state.active_field_mut().push_str("/home/user/.ssh/id_rsa");
    assert_eq!(state.key_path, "/home/user/.ssh/id_rsa");
}

#[test]
fn test_active_field_mut_fallback() {
    let mut state = ConnectDialogState::new();
    state.focused_field = 0; // protocol field — returns host as fallback
    state.active_field_mut().push_str("fallback");
    assert_eq!(state.host, "fallback");
}

// ==================== Dialog event handler tests ====================

#[test]
fn test_esc_closes_dialog() {
    let mut ctx = create_context_with_dialog();
    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Esc)).unwrap();
    assert!(ctx.connect_dialog.is_none());
    assert_eq!(ctx.get_ui_state(), Some(UiState::Normal));
}

#[test]
fn test_tab_advances_field() {
    let mut ctx = create_context_with_dialog();
    assert_eq!(ctx.connect_dialog.as_ref().unwrap().focused_field, 1); // host

    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Tab)).unwrap();
    assert_eq!(ctx.connect_dialog.as_ref().unwrap().focused_field, 2); // port

    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Tab)).unwrap();
    assert_eq!(ctx.connect_dialog.as_ref().unwrap().focused_field, 3); // username
}

#[test]
fn test_tab_wraps_around() {
    let mut ctx = create_context_with_dialog();
    ctx.connect_dialog.as_mut().unwrap().focused_field = 5; // last field (key_path)

    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Tab)).unwrap();
    assert_eq!(ctx.connect_dialog.as_ref().unwrap().focused_field, 0); // wraps to protocol
}

#[test]
fn test_backtab_goes_backward() {
    let mut ctx = create_context_with_dialog();
    ctx.connect_dialog.as_mut().unwrap().focused_field = 3; // username

    event_handler::handle_connect_dialog_event(&mut ctx, key_shift_tab()).unwrap();
    assert_eq!(ctx.connect_dialog.as_ref().unwrap().focused_field, 2); // port
}

#[test]
fn test_backtab_wraps_from_zero() {
    let mut ctx = create_context_with_dialog();
    ctx.connect_dialog.as_mut().unwrap().focused_field = 0;

    event_handler::handle_connect_dialog_event(&mut ctx, key_shift_tab()).unwrap();
    assert_eq!(ctx.connect_dialog.as_ref().unwrap().focused_field, 5); // wraps to key_path
}

#[test]
fn test_protocol_toggle_up() {
    let mut ctx = create_context_with_dialog();
    ctx.connect_dialog.as_mut().unwrap().focused_field = 0;
    assert_eq!(
        ctx.connect_dialog.as_ref().unwrap().protocol,
        ConnectionProtocol::Sftp
    );
    assert_eq!(ctx.connect_dialog.as_ref().unwrap().port, "22");

    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Up)).unwrap();
    assert_eq!(
        ctx.connect_dialog.as_ref().unwrap().protocol,
        ConnectionProtocol::Ftp
    );
    assert_eq!(ctx.connect_dialog.as_ref().unwrap().port, "21");
}

#[test]
fn test_protocol_toggle_down() {
    let mut ctx = create_context_with_dialog();
    ctx.connect_dialog.as_mut().unwrap().focused_field = 0;

    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Down)).unwrap();
    assert_eq!(
        ctx.connect_dialog.as_ref().unwrap().protocol,
        ConnectionProtocol::Ftp
    );

    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Down)).unwrap();
    assert_eq!(
        ctx.connect_dialog.as_ref().unwrap().protocol,
        ConnectionProtocol::Sftp
    );
    assert_eq!(ctx.connect_dialog.as_ref().unwrap().port, "22");
}

#[test]
fn test_char_input_on_host() {
    let mut ctx = create_context_with_dialog();
    // focused_field starts at 1 (host)

    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Char('a'))).unwrap();
    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Char('b'))).unwrap();
    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Char('c'))).unwrap();

    assert_eq!(ctx.connect_dialog.as_ref().unwrap().host, "abc");
}

#[test]
fn test_backspace_on_field() {
    let mut ctx = create_context_with_dialog();
    ctx.connect_dialog.as_mut().unwrap().host = "test".to_string();

    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Backspace)).unwrap();
    assert_eq!(ctx.connect_dialog.as_ref().unwrap().host, "tes");

    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Backspace)).unwrap();
    assert_eq!(ctx.connect_dialog.as_ref().unwrap().host, "te");
}

#[test]
fn test_backspace_on_empty_field() {
    let mut ctx = create_context_with_dialog();
    assert_eq!(ctx.connect_dialog.as_ref().unwrap().host, "");

    // Should not panic
    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Backspace)).unwrap();
    assert_eq!(ctx.connect_dialog.as_ref().unwrap().host, "");
}

#[test]
fn test_char_ignored_on_protocol_field() {
    let mut ctx = create_context_with_dialog();
    ctx.connect_dialog.as_mut().unwrap().focused_field = 0;

    // Char on protocol field should do nothing (guard: focused_field > 0)
    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Char('x'))).unwrap();
    assert_eq!(ctx.connect_dialog.as_ref().unwrap().host, ""); // unchanged
}

#[test]
fn test_enter_with_empty_host_shows_error() {
    let mut ctx = create_context_with_dialog();
    // host is empty by default

    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Enter)).unwrap();

    // Dialog should still be open with error
    assert!(ctx.connect_dialog.is_some());
    assert_eq!(ctx.get_ui_state(), Some(UiState::ConnectDialog));
    assert_eq!(
        ctx.connect_dialog.as_ref().unwrap().error_message,
        Some("Host is required".to_string())
    );
}

#[test]
fn test_enter_with_host_attempts_connection() {
    let mut ctx = create_context_with_dialog();
    ctx.connect_dialog.as_mut().unwrap().host = "nonexistent.invalid".to_string();
    ctx.connect_dialog.as_mut().unwrap().username = "user".to_string();

    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Enter)).unwrap();

    // Connection should fail — dialog stays open with error
    assert!(ctx.connect_dialog.is_some());
    let err = ctx
        .connect_dialog
        .as_ref()
        .unwrap()
        .error_message
        .as_ref()
        .unwrap();
    assert!(!err.is_empty());
}

#[test]
fn test_password_input() {
    let mut ctx = create_context_with_dialog();
    // Navigate to password field
    ctx.connect_dialog.as_mut().unwrap().focused_field = 4;

    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Char('s'))).unwrap();
    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Char('e'))).unwrap();
    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Char('c'))).unwrap();

    assert_eq!(ctx.connect_dialog.as_ref().unwrap().password, "sec");
}

#[test]
fn test_full_tab_cycle() {
    let mut ctx = create_context_with_dialog();
    let count = ctx.connect_dialog.as_ref().unwrap().field_count();

    // Tab through all fields and back to start
    for _ in 0..count {
        event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Tab)).unwrap();
    }
    // Should wrap back to where we started +6 = field 1 (since 1+6 % 6 = 1)
    assert_eq!(ctx.connect_dialog.as_ref().unwrap().focused_field, 1);
}

#[test]
fn test_f5_opens_dialog() {
    let mut ctx = Context::new().unwrap();
    assert!(ctx.connect_dialog.is_none());
    assert_eq!(ctx.get_ui_state(), Some(UiState::Normal));

    event_handler::handle_main_event(&mut ctx, key(KeyCode::F(5))).unwrap();

    assert!(ctx.connect_dialog.is_some());
    assert_eq!(ctx.get_ui_state(), Some(UiState::ConnectDialog));
}

#[test]
fn test_no_dialog_resets_to_normal() {
    let mut ctx = Context::new().unwrap();
    ctx.set_ui_state(UiState::ConnectDialog);
    ctx.connect_dialog = None; // no dialog state

    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Char('a'))).unwrap();
    assert_eq!(ctx.get_ui_state(), Some(UiState::Normal));
}

// ==================== FTP parse_list_line tests ====================

#[cfg(feature = "ftp")]
mod ftp_tests {
    use vocofo::ftp_backend::parse_list_line;

    #[test]
    fn test_parse_regular_file() {
        let line = "-rw-r--r-- 1 user group 12345 Jan 15 10:30 document.txt";
        let entry = parse_list_line(line).unwrap();
        assert_eq!(entry.name, "document.txt");
        assert!(entry.info.is_file);
        assert!(!entry.info.is_dir);
        assert_eq!(entry.info.size, 12345);
    }

    #[test]
    fn test_parse_directory() {
        let line = "drwxr-xr-x 2 user group 4096 Mar 10 08:00 projects";
        let entry = parse_list_line(line).unwrap();
        assert_eq!(entry.name, "projects");
        assert!(entry.info.is_dir);
        assert!(!entry.info.is_file);
    }

    #[test]
    fn test_parse_symlink() {
        let line = "lrwxrwxrwx 1 user group 11 Feb 20 14:00 link -> target.txt";
        let entry = parse_list_line(line).unwrap();
        assert_eq!(entry.name, "link");
        assert!(entry.info.is_symlink);
        assert!(!entry.info.is_dir);
        assert!(!entry.info.is_file);
    }

    #[test]
    fn test_parse_file_with_spaces_in_name() {
        let line = "-rw-r--r-- 1 user group 100 Jan 01 12:00 my file name.txt";
        let entry = parse_list_line(line).unwrap();
        assert_eq!(entry.name, "my file name.txt");
    }

    #[test]
    fn test_parse_skips_dot() {
        let line = "drwxr-xr-x 2 user group 4096 Jan 01 12:00 .";
        assert!(parse_list_line(line).is_none());
    }

    #[test]
    fn test_parse_skips_dotdot() {
        let line = "drwxr-xr-x 2 user group 4096 Jan 01 12:00 ..";
        assert!(parse_list_line(line).is_none());
    }

    #[test]
    fn test_parse_short_line_returns_none() {
        let line = "drwx only four parts";
        assert!(parse_list_line(line).is_none());
    }

    #[test]
    fn test_parse_empty_line_returns_none() {
        assert!(parse_list_line("").is_none());
    }

    #[test]
    fn test_parse_invalid_size() {
        let line = "-rw-r--r-- 1 user group notanumber Jan 15 10:30 file.txt";
        let entry = parse_list_line(line).unwrap();
        assert_eq!(entry.info.size, 0); // falls back to 0
    }

    #[test]
    fn test_parse_large_file() {
        let line = "-rw-r--r-- 1 user group 1073741824 Jan 15 10:30 bigfile.iso";
        let entry = parse_list_line(line).unwrap();
        assert_eq!(entry.info.size, 1073741824);
        assert_eq!(entry.name, "bigfile.iso");
    }
}

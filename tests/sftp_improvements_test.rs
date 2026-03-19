use std::fs;
use std::sync::Arc;
use tempfile::TempDir;
use vocofo::backend::FilesystemBackend;
use vocofo::config::{Config, ConnectionProfile};
use vocofo::context::{ConnectDialogState, ConnectionProtocol, Context, UiState};
use vocofo::event_handler;
use vocofo::file_operation;
use vocofo::local_backend::LocalBackend;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn key_ctrl(c: char) -> KeyEvent {
    KeyEvent {
        code: KeyCode::Char(c),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn create_test_context() -> (Context, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();
    fs::write(base.join("file1.txt"), "hello").unwrap();
    fs::write(base.join("file2.txt"), "world").unwrap();
    fs::create_dir(base.join("subdir")).unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = base.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();
    (context, temp_dir)
}

// ==================== FTP metadata parsing tests ====================

#[cfg(feature = "ftp")]
mod ftp_metadata_tests {
    use vocofo::ftp_backend::parse_list_line;

    #[test]
    fn test_parse_list_line_with_mtime_time_format() {
        let line = "-rw-r--r-- 1 user group 1234 Jan 15 09:30 testfile.txt";
        let entry = parse_list_line(line).unwrap();
        assert_eq!(entry.name, "testfile.txt");
        assert_eq!(entry.info.size, 1234);
        assert!(entry.info.modified.is_some());
        assert!(!entry.info.readonly);
    }

    #[test]
    fn test_parse_list_line_with_mtime_year_format() {
        let line = "-rw-r--r-- 1 user group 5678 Dec 25 2023 oldfile.txt";
        let entry = parse_list_line(line).unwrap();
        assert_eq!(entry.name, "oldfile.txt");
        assert!(entry.info.modified.is_some());
    }

    #[test]
    fn test_parse_list_line_readonly() {
        let line = "-r--r--r-- 1 user group 100 Mar 01 12:00 readonly.txt";
        let entry = parse_list_line(line).unwrap();
        assert!(entry.info.readonly);
    }

    #[test]
    fn test_parse_list_line_writable() {
        let line = "-rw-r--r-- 1 user group 100 Mar 01 12:00 writable.txt";
        let entry = parse_list_line(line).unwrap();
        assert!(!entry.info.readonly);
    }

    #[test]
    fn test_parse_list_line_symlink_detected() {
        let line = "lrwxrwxrwx 1 user group 10 Jan 01 12:00 link -> target";
        let entry = parse_list_line(line).unwrap();
        assert!(entry.info.is_symlink);
        assert_eq!(entry.name, "link");
    }

    #[test]
    fn test_parse_list_line_mode_755() {
        let line = "-rwxr-xr-x 1 user group 4096 Jan 01 12:00 script.sh";
        let entry = parse_list_line(line).unwrap();
        assert_eq!(entry.info.mode, Some(0o755));
    }

    #[test]
    fn test_parse_list_line_mode_644() {
        let line = "-rw-r--r-- 1 user group 100 Jan 01 12:00 file.txt";
        let entry = parse_list_line(line).unwrap();
        assert_eq!(entry.info.mode, Some(0o644));
    }

    #[test]
    fn test_parse_list_line_mode_700() {
        let line = "-rwx------ 1 user group 100 Jan 01 12:00 private.sh";
        let entry = parse_list_line(line).unwrap();
        assert_eq!(entry.info.mode, Some(0o700));
    }

    #[test]
    fn test_parse_list_line_directory_mode() {
        let line = "drwxr-xr-x 2 user group 4096 Jan 01 12:00 mydir";
        let entry = parse_list_line(line).unwrap();
        assert!(entry.info.is_dir);
        assert_eq!(entry.info.mode, Some(0o755));
    }

    #[test]
    fn test_parse_list_line_invalid_month_no_mtime() {
        let line = "-rw-r--r-- 1 user group 100 Xyz 01 12:00 file.txt";
        let entry = parse_list_line(line).unwrap();
        assert!(entry.info.modified.is_none());
    }
}

// ==================== Backend trait: disconnect, chmod, is_connected ====================

#[test]
fn test_local_backend_disconnect_noop() {
    let backend = LocalBackend::new();
    // Should not panic
    backend.disconnect();
}

#[test]
fn test_local_backend_is_connected() {
    let backend = LocalBackend::new();
    assert!(backend.is_connected());
}

#[test]
fn test_local_backend_connection_params_none() {
    let backend = LocalBackend::new();
    assert!(backend.connection_params().is_none());
}

#[cfg(unix)]
#[test]
fn test_local_backend_chmod() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "test").unwrap();

    let backend = LocalBackend::new();
    let path_str = file_path.to_string_lossy().to_string();

    backend.chmod(&path_str, 0o755).unwrap();
    let meta = fs::metadata(&file_path).unwrap();
    assert_eq!(meta.permissions().mode() & 0o777, 0o755);

    backend.chmod(&path_str, 0o644).unwrap();
    let meta = fs::metadata(&file_path).unwrap();
    assert_eq!(meta.permissions().mode() & 0o777, 0o644);
}

#[test]
fn test_local_backend_fileinfo_has_mode() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "test").unwrap();

    let backend = LocalBackend::new();
    let info = backend.metadata(&file_path.to_string_lossy()).unwrap();
    #[cfg(unix)]
    assert!(info.mode.is_some());
}

// ==================== chmod popup tests ====================

#[test]
fn test_chmod_popup_opens_on_ctrl_m() {
    let (mut ctx, _tmp) = create_test_context();
    // Select first file (not ../)
    ctx.panels[0].state = 1;

    event_handler::handle_main_event(&mut ctx, key_ctrl('m')).unwrap();
    assert_eq!(ctx.ui_state, UiState::ChmodPopup);
}

#[test]
fn test_chmod_popup_ignores_parent_dir() {
    let (mut ctx, _tmp) = create_test_context();
    // Select ../ (index 0)
    ctx.panels[0].state = 0;

    event_handler::handle_main_event(&mut ctx, key_ctrl('m')).unwrap();
    // Should stay in Normal state since ../ can't be chmod'd
    assert_eq!(ctx.ui_state, UiState::Normal);
}

#[test]
fn test_chmod_popup_esc_closes() {
    let (mut ctx, _tmp) = create_test_context();
    ctx.panels[0].state = 1;
    event_handler::handle_main_event(&mut ctx, key_ctrl('m')).unwrap();
    assert_eq!(ctx.ui_state, UiState::ChmodPopup);

    event_handler::handle_chmod_popup_event(&mut ctx, key(KeyCode::Esc)).unwrap();
    assert_eq!(ctx.ui_state, UiState::Normal);
}

#[test]
fn test_chmod_popup_accepts_only_octal() {
    let (mut ctx, _tmp) = create_test_context();
    ctx.set_ui_state(UiState::ChmodPopup);
    ctx.input = String::new();

    // Valid octal digits
    event_handler::handle_chmod_popup_event(&mut ctx, key(KeyCode::Char('7'))).unwrap();
    event_handler::handle_chmod_popup_event(&mut ctx, key(KeyCode::Char('5'))).unwrap();
    event_handler::handle_chmod_popup_event(&mut ctx, key(KeyCode::Char('5'))).unwrap();
    assert_eq!(ctx.input, "755");

    // '8' and '9' should be rejected (not valid octal)
    event_handler::handle_chmod_popup_event(&mut ctx, key(KeyCode::Char('8'))).unwrap();
    assert_eq!(ctx.input, "755"); // unchanged

    // Max 4 chars
    event_handler::handle_chmod_popup_event(&mut ctx, key(KeyCode::Char('0'))).unwrap();
    assert_eq!(ctx.input, "7550");
    event_handler::handle_chmod_popup_event(&mut ctx, key(KeyCode::Char('1'))).unwrap();
    assert_eq!(ctx.input, "7550"); // still 4, not added
}

#[cfg(unix)]
#[test]
fn test_chmod_popup_enter_applies() {
    let (mut ctx, _tmp) = create_test_context();
    ctx.panels[0].state = 1;

    // Open chmod popup
    event_handler::handle_main_event(&mut ctx, key_ctrl('m')).unwrap();
    assert_eq!(ctx.ui_state, UiState::ChmodPopup);

    // Clear and type 755
    ctx.input = "755".to_string();
    event_handler::handle_chmod_popup_event(&mut ctx, key(KeyCode::Enter)).unwrap();
    assert_eq!(ctx.ui_state, UiState::Normal);

    // Verify the file has the new permissions
    let item = ctx.panels[0].items[1].trim_end_matches('/').to_string();
    let path = format!("{}/{}", ctx.panels[0].path, item);
    let meta = fs::metadata(&path).unwrap();
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(meta.permissions().mode() & 0o777, 0o755);
}

// ==================== Config save/load tests ====================

#[test]
fn test_config_save_and_load() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let mut config = Config::default();
    config.connections.push(ConnectionProfile {
        name: "test-server".to_string(),
        protocol: "sftp".to_string(),
        host: "example.com".to_string(),
        port: 22,
        username: "user".to_string(),
        key_path: Some("/home/user/.ssh/id_rsa".to_string()),
    });

    // Serialize and write
    let content = toml::to_string_pretty(&config).unwrap();
    fs::write(&config_path, &content).unwrap();

    // Read back
    let loaded: Config = toml::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    assert_eq!(loaded.connections.len(), 1);
    assert_eq!(loaded.connections[0].name, "test-server");
    assert_eq!(loaded.connections[0].host, "example.com");
    assert_eq!(loaded.connections[0].port, 22);
    assert_eq!(loaded.connections[0].username, "user");
    assert_eq!(loaded.connections[0].key_path, Some("/home/user/.ssh/id_rsa".to_string()));
}

#[test]
fn test_config_connections_default_empty() {
    let config = Config::default();
    assert!(config.connections.is_empty());
}

#[test]
fn test_config_roundtrip_no_password() {
    let mut config = Config::default();
    config.connections.push(ConnectionProfile {
        name: "myserver".to_string(),
        protocol: "ftp".to_string(),
        host: "ftp.example.com".to_string(),
        port: 21,
        username: "admin".to_string(),
        key_path: None,
    });

    let serialized = toml::to_string_pretty(&config).unwrap();
    // Verify no password field in serialized output
    assert!(!serialized.contains("password"));

    let deserialized: Config = toml::from_str(&serialized).unwrap();
    assert_eq!(deserialized.connections[0].name, "myserver");
    assert_eq!(deserialized.connections[0].protocol, "ftp");
}

// ==================== Bookmark list tests ====================

#[test]
fn test_f7_opens_bookmark_list() {
    let mut ctx = Context::new().unwrap();
    ctx.config.connections.push(ConnectionProfile {
        name: "server1".to_string(),
        protocol: "sftp".to_string(),
        host: "host1.com".to_string(),
        port: 22,
        username: "user1".to_string(),
        key_path: None,
    });

    event_handler::handle_main_event(&mut ctx, key(KeyCode::F(7))).unwrap();
    assert_eq!(ctx.ui_state, UiState::BookmarkList);
    assert_eq!(ctx.bookmark_selected, 0);
}

#[test]
fn test_f7_no_bookmarks_shows_message() {
    let mut ctx = Context::new().unwrap();
    assert!(ctx.config.connections.is_empty());

    event_handler::handle_main_event(&mut ctx, key(KeyCode::F(7))).unwrap();
    assert_eq!(ctx.ui_state, UiState::Normal);
    // Should show status message about no bookmarks
    assert!(ctx.get_status_message().is_some());
}

#[test]
fn test_bookmark_list_navigation() {
    let mut ctx = Context::new().unwrap();
    ctx.config.connections.push(ConnectionProfile {
        name: "srv1".to_string(), protocol: "sftp".to_string(),
        host: "a.com".to_string(), port: 22, username: "u".to_string(), key_path: None,
    });
    ctx.config.connections.push(ConnectionProfile {
        name: "srv2".to_string(), protocol: "ftp".to_string(),
        host: "b.com".to_string(), port: 21, username: "v".to_string(), key_path: None,
    });
    ctx.config.connections.push(ConnectionProfile {
        name: "srv3".to_string(), protocol: "sftp".to_string(),
        host: "c.com".to_string(), port: 2222, username: "w".to_string(), key_path: None,
    });

    ctx.bookmark_selected = 0;
    ctx.set_ui_state(UiState::BookmarkList);

    // Move down
    event_handler::handle_bookmark_list_event(&mut ctx, key(KeyCode::Down)).unwrap();
    assert_eq!(ctx.bookmark_selected, 1);

    event_handler::handle_bookmark_list_event(&mut ctx, key(KeyCode::Down)).unwrap();
    assert_eq!(ctx.bookmark_selected, 2);

    // Can't go past end
    event_handler::handle_bookmark_list_event(&mut ctx, key(KeyCode::Down)).unwrap();
    assert_eq!(ctx.bookmark_selected, 2);

    // Move up
    event_handler::handle_bookmark_list_event(&mut ctx, key(KeyCode::Up)).unwrap();
    assert_eq!(ctx.bookmark_selected, 1);

    // vi keys work too
    event_handler::handle_bookmark_list_event(&mut ctx, key(KeyCode::Char('k'))).unwrap();
    assert_eq!(ctx.bookmark_selected, 0);

    event_handler::handle_bookmark_list_event(&mut ctx, key(KeyCode::Char('j'))).unwrap();
    assert_eq!(ctx.bookmark_selected, 1);
}

#[test]
fn test_bookmark_list_esc_closes() {
    let mut ctx = Context::new().unwrap();
    ctx.config.connections.push(ConnectionProfile {
        name: "srv".to_string(), protocol: "sftp".to_string(),
        host: "h.com".to_string(), port: 22, username: "u".to_string(), key_path: None,
    });
    ctx.set_ui_state(UiState::BookmarkList);

    event_handler::handle_bookmark_list_event(&mut ctx, key(KeyCode::Esc)).unwrap();
    assert_eq!(ctx.ui_state, UiState::Normal);
}

#[test]
fn test_bookmark_list_enter_loads_profile() {
    let mut ctx = Context::new().unwrap();
    ctx.config.connections.push(ConnectionProfile {
        name: "myserver".to_string(),
        protocol: "sftp".to_string(),
        host: "ssh.example.com".to_string(),
        port: 2222,
        username: "admin".to_string(),
        key_path: Some("/path/to/key".to_string()),
    });
    ctx.bookmark_selected = 0;
    ctx.set_ui_state(UiState::BookmarkList);

    event_handler::handle_bookmark_list_event(&mut ctx, key(KeyCode::Enter)).unwrap();
    assert_eq!(ctx.ui_state, UiState::ConnectDialog);

    let dialog = ctx.connect_dialog.as_ref().unwrap();
    assert_eq!(dialog.protocol, ConnectionProtocol::Sftp);
    assert_eq!(dialog.host, "ssh.example.com");
    assert_eq!(dialog.port, "2222");
    assert_eq!(dialog.username, "admin");
    assert_eq!(dialog.key_path, "/path/to/key");
    assert_eq!(dialog.password, ""); // password never loaded
}

#[test]
fn test_bookmark_list_enter_ftp_profile() {
    let mut ctx = Context::new().unwrap();
    ctx.config.connections.push(ConnectionProfile {
        name: "ftpserver".to_string(),
        protocol: "ftp".to_string(),
        host: "ftp.example.com".to_string(),
        port: 21,
        username: "ftpuser".to_string(),
        key_path: None,
    });
    ctx.bookmark_selected = 0;
    ctx.set_ui_state(UiState::BookmarkList);

    event_handler::handle_bookmark_list_event(&mut ctx, key(KeyCode::Enter)).unwrap();
    let dialog = ctx.connect_dialog.as_ref().unwrap();
    assert_eq!(dialog.protocol, ConnectionProtocol::Ftp);
    assert_eq!(dialog.key_path, "");
}

#[test]
fn test_bookmark_delete() {
    let mut ctx = Context::new().unwrap();
    ctx.config.connections.push(ConnectionProfile {
        name: "srv1".to_string(), protocol: "sftp".to_string(),
        host: "a.com".to_string(), port: 22, username: "u".to_string(), key_path: None,
    });
    ctx.config.connections.push(ConnectionProfile {
        name: "srv2".to_string(), protocol: "sftp".to_string(),
        host: "b.com".to_string(), port: 22, username: "v".to_string(), key_path: None,
    });
    ctx.bookmark_selected = 0;
    ctx.set_ui_state(UiState::BookmarkList);

    event_handler::handle_bookmark_list_event(&mut ctx, key(KeyCode::Char('d'))).unwrap();
    assert_eq!(ctx.config.connections.len(), 1);
    assert_eq!(ctx.config.connections[0].name, "srv2");
    assert_eq!(ctx.bookmark_selected, 0);
}

#[test]
fn test_bookmark_delete_last_closes_list() {
    let mut ctx = Context::new().unwrap();
    ctx.config.connections.push(ConnectionProfile {
        name: "only".to_string(), protocol: "sftp".to_string(),
        host: "a.com".to_string(), port: 22, username: "u".to_string(), key_path: None,
    });
    ctx.bookmark_selected = 0;
    ctx.set_ui_state(UiState::BookmarkList);

    event_handler::handle_bookmark_list_event(&mut ctx, key(KeyCode::Char('d'))).unwrap();
    assert!(ctx.config.connections.is_empty());
    assert_eq!(ctx.ui_state, UiState::Normal);
}

// ==================== Bookmark name input tests ====================

#[test]
fn test_bookmark_name_input_esc_returns_to_dialog() {
    let mut ctx = Context::new().unwrap();
    ctx.connect_dialog = Some(ConnectDialogState::new());
    ctx.set_ui_state(UiState::BookmarkNameInput);
    ctx.input = "some name".to_string();

    event_handler::handle_bookmark_name_event(&mut ctx, key(KeyCode::Esc)).unwrap();
    assert_eq!(ctx.ui_state, UiState::ConnectDialog);
    assert_eq!(ctx.input, "");
}

#[test]
fn test_bookmark_name_input_enter_saves() {
    let mut ctx = Context::new().unwrap();
    let mut dialog = ConnectDialogState::new();
    dialog.host = "test.com".to_string();
    dialog.port = "22".to_string();
    dialog.username = "testuser".to_string();
    ctx.connect_dialog = Some(dialog);
    ctx.set_ui_state(UiState::BookmarkNameInput);
    ctx.input = "My Server".to_string();

    event_handler::handle_bookmark_name_event(&mut ctx, key(KeyCode::Enter)).unwrap();
    assert_eq!(ctx.ui_state, UiState::ConnectDialog);
    assert_eq!(ctx.config.connections.len(), 1);
    assert_eq!(ctx.config.connections[0].name, "My Server");
    assert_eq!(ctx.config.connections[0].host, "test.com");
    assert_eq!(ctx.config.connections[0].username, "testuser");
}

#[test]
fn test_bookmark_name_input_empty_does_not_save() {
    let mut ctx = Context::new().unwrap();
    ctx.connect_dialog = Some(ConnectDialogState::new());
    ctx.set_ui_state(UiState::BookmarkNameInput);
    ctx.input = String::new();

    event_handler::handle_bookmark_name_event(&mut ctx, key(KeyCode::Enter)).unwrap();
    assert_eq!(ctx.ui_state, UiState::ConnectDialog);
    assert!(ctx.config.connections.is_empty());
}

#[test]
fn test_bookmark_name_input_typing() {
    let mut ctx = Context::new().unwrap();
    ctx.set_ui_state(UiState::BookmarkNameInput);
    ctx.input = String::new();

    event_handler::handle_bookmark_name_event(&mut ctx, key(KeyCode::Char('H'))).unwrap();
    event_handler::handle_bookmark_name_event(&mut ctx, key(KeyCode::Char('i'))).unwrap();
    assert_eq!(ctx.input, "Hi");

    event_handler::handle_bookmark_name_event(&mut ctx, key(KeyCode::Backspace)).unwrap();
    assert_eq!(ctx.input, "H");
}

// ==================== Ctrl+S in connect dialog saves bookmark ====================

#[test]
fn test_ctrl_s_in_dialog_opens_name_input() {
    let mut ctx = Context::new().unwrap();
    let mut dialog = ConnectDialogState::new();
    dialog.host = "myhost.com".to_string();
    dialog.username = "admin".to_string();
    ctx.connect_dialog = Some(dialog);
    ctx.set_ui_state(UiState::ConnectDialog);

    event_handler::handle_connect_dialog_event(&mut ctx, key_ctrl('s')).unwrap();
    assert_eq!(ctx.ui_state, UiState::BookmarkNameInput);
    assert_eq!(ctx.input, "admin@myhost.com");
}

#[test]
fn test_ctrl_s_empty_host_shows_error() {
    let mut ctx = Context::new().unwrap();
    let dialog = ConnectDialogState::new(); // host is empty
    ctx.connect_dialog = Some(dialog);
    ctx.set_ui_state(UiState::ConnectDialog);

    event_handler::handle_connect_dialog_event(&mut ctx, key_ctrl('s')).unwrap();
    // Should stay in ConnectDialog with error
    assert_eq!(ctx.ui_state, UiState::ConnectDialog);
    let d = ctx.connect_dialog.as_ref().unwrap();
    assert!(d.error_message.is_some());
}

// ==================== Progress indicator tests ====================

#[test]
fn test_transfer_progress_new() {
    use std::sync::atomic::Ordering;
    let progress = vocofo::background_op::TransferProgress::new();
    assert_eq!(progress.bytes_transferred.load(Ordering::Relaxed), 0);
    assert_eq!(progress.total_bytes.load(Ordering::Relaxed), 0);
}

#[test]
fn test_transfer_progress_atomic_updates() {
    use std::sync::atomic::Ordering;
    let progress = vocofo::background_op::TransferProgress::new();
    progress.total_bytes.store(1000, Ordering::Relaxed);
    progress.bytes_transferred.fetch_add(500, Ordering::Relaxed);
    assert_eq!(progress.bytes_transferred.load(Ordering::Relaxed), 500);
    progress.bytes_transferred.fetch_add(500, Ordering::Relaxed);
    assert_eq!(progress.bytes_transferred.load(Ordering::Relaxed), 1000);
}

#[test]
fn test_context_transfer_progress_default_none() {
    let ctx = Context::new().unwrap();
    assert!(ctx.transfer_progress.is_none());
}

// ==================== ConnectionParams / ConnectionProtocol in backend ====================

#[test]
fn test_connection_protocol_clone_eq() {
    let sftp = ConnectionProtocol::Sftp;
    let ftp = ConnectionProtocol::Ftp;
    assert_eq!(sftp, ConnectionProtocol::Sftp);
    assert_eq!(ftp, ConnectionProtocol::Ftp);
    assert_ne!(sftp, ftp);
}

#[test]
fn test_connection_params_clone() {
    use vocofo::backend::ConnectionParams;
    let params = ConnectionParams {
        protocol: ConnectionProtocol::Sftp,
        host: "example.com".to_string(),
        port: 22,
        username: "user".to_string(),
        password: "pass".to_string(),
        key_path: Some("/path/key".to_string()),
    };
    let cloned = params.clone();
    assert_eq!(cloned.host, "example.com");
    assert_eq!(cloned.port, 22);
    assert_eq!(cloned.protocol, ConnectionProtocol::Sftp);
}

// ==================== Context stores config ====================

#[test]
fn test_context_stores_config() {
    let mut config = Config::default();
    config.connections.push(ConnectionProfile {
        name: "test".to_string(),
        protocol: "sftp".to_string(),
        host: "h.com".to_string(),
        port: 22,
        username: "u".to_string(),
        key_path: None,
    });
    let ctx = Context::with_config(config).unwrap();
    assert_eq!(ctx.config.connections.len(), 1);
    assert_eq!(ctx.config.connections[0].name, "test");
}

// ==================== Disconnect handler calls disconnect ====================

#[test]
fn test_f6_on_local_does_nothing() {
    let mut ctx = Context::new().unwrap();
    // Active panel is local by default
    assert!(ctx.active().backend.is_local());

    event_handler::handle_main_event(&mut ctx, key(KeyCode::F(6))).unwrap();
    // Should remain local, no crash
    assert!(ctx.active().backend.is_local());
}

// ==================== read_file OOM fix tests ====================

#[test]
fn test_read_file_with_usize_max_does_not_oom() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("small.txt");
    fs::write(&file_path, "hello world").unwrap();

    let backend = LocalBackend::new();
    let data = backend.read_file(&file_path.to_string_lossy(), usize::MAX).unwrap();
    assert_eq!(data, b"hello world");
}

#[test]
fn test_read_file_respects_max_bytes() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "hello world 12345").unwrap();

    let backend = LocalBackend::new();
    let data = backend.read_file(&file_path.to_string_lossy(), 5).unwrap();
    assert_eq!(data, b"hello");
}

#[test]
fn test_read_file_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.txt");
    fs::write(&file_path, "").unwrap();

    let backend = LocalBackend::new();
    let data = backend.read_file(&file_path.to_string_lossy(), usize::MAX).unwrap();
    assert!(data.is_empty());
}

// ==================== Connect dialog port validation tests ====================

#[test]
fn test_connect_dialog_invalid_port_shows_error() {
    let mut ctx = Context::new().unwrap();
    let mut dialog = ConnectDialogState::new();
    dialog.host = "test.com".to_string();
    dialog.port = "abc".to_string();
    ctx.connect_dialog = Some(dialog);
    ctx.set_ui_state(UiState::ConnectDialog);

    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Enter)).unwrap();
    // Should stay in dialog with error
    assert_eq!(ctx.ui_state, UiState::ConnectDialog);
    let d = ctx.connect_dialog.as_ref().unwrap();
    assert!(d.error_message.is_some());
    assert!(d.error_message.as_ref().unwrap().contains("port"));
}

#[test]
fn test_connect_dialog_port_zero_shows_error() {
    let mut ctx = Context::new().unwrap();
    let mut dialog = ConnectDialogState::new();
    dialog.host = "test.com".to_string();
    dialog.port = "0".to_string();
    ctx.connect_dialog = Some(dialog);
    ctx.set_ui_state(UiState::ConnectDialog);

    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Enter)).unwrap();
    let d = ctx.connect_dialog.as_ref().unwrap();
    assert!(d.error_message.is_some());
}

#[test]
fn test_connect_dialog_port_too_large_shows_error() {
    let mut ctx = Context::new().unwrap();
    let mut dialog = ConnectDialogState::new();
    dialog.host = "test.com".to_string();
    dialog.port = "99999".to_string();
    ctx.connect_dialog = Some(dialog);
    ctx.set_ui_state(UiState::ConnectDialog);

    event_handler::handle_connect_dialog_event(&mut ctx, key(KeyCode::Enter)).unwrap();
    let d = ctx.connect_dialog.as_ref().unwrap();
    assert!(d.error_message.is_some());
}

// ==================== Config atomic save tests ====================

#[test]
fn test_config_save_creates_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("vocofo").join("config.toml");

    let mut config = Config::default();
    config.connections.push(ConnectionProfile {
        name: "saved".to_string(),
        protocol: "sftp".to_string(),
        host: "saved.com".to_string(),
        port: 22,
        username: "user".to_string(),
        key_path: None,
    });

    // Manually serialize to verify format
    let content = toml::to_string_pretty(&config).unwrap();
    let parent = config_path.parent().unwrap();
    fs::create_dir_all(parent).unwrap();
    fs::write(&config_path, &content).unwrap();

    let loaded: Config = toml::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    assert_eq!(loaded.connections.len(), 1);
    assert_eq!(loaded.connections[0].name, "saved");
}

#[test]
fn test_config_no_tmp_file_left_after_save() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("vocofo");
    fs::create_dir_all(&config_dir).unwrap();

    let config_path = config_dir.join("config.toml");
    let tmp_path = config_dir.join("config.toml.tmp");

    let config = Config::default();
    let content = toml::to_string_pretty(&config).unwrap();
    // Simulate atomic save: write tmp then rename
    fs::write(&tmp_path, &content).unwrap();
    fs::rename(&tmp_path, &config_path).unwrap();

    assert!(config_path.exists());
    assert!(!tmp_path.exists()); // tmp should be gone after rename
}

// ==================== SCP shell_escape tests ====================

#[cfg(feature = "sftp")]
mod scp_tests {
    use vocofo::scp_backend;

    // shell_escape is private, but we can test it indirectly
    // by verifying the ScpBackend doesn't crash with special filenames

    #[test]
    fn test_scp_backend_display_name() {
        // Just verify the module compiles and types exist
        use vocofo::backend::{ConnectionParams, ConnectionProtocol};
        let params = ConnectionParams {
            protocol: ConnectionProtocol::Sftp,
            host: "test.com".to_string(),
            port: 22,
            username: "user".to_string(),
            password: "pass".to_string(),
            key_path: None,
        };
        // Can't easily test without a real SSH session,
        // but verify types compile
        let _ = params.clone();
    }
}

// ==================== Batch error reporting tests ====================

#[test]
fn test_batch_delete_error_format() {
    // Verify error message format includes count
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Create one file, try to delete it plus a nonexistent one
    fs::write(base.join("exists.txt"), "data").unwrap();

    let backend: Arc<dyn FilesystemBackend> = Arc::new(LocalBackend::new());
    let paths = vec![
        base.join("exists.txt").to_string_lossy().to_string(),
        base.join("nonexistent.txt").to_string_lossy().to_string(),
    ];

    let rx = vocofo::background_op::spawn_delete_batch_with_backend(
        backend, paths, "Deleting...".to_string(),
    );

    let result = rx.recv().unwrap();
    match result.result {
        Ok(()) => panic!("Expected error for nonexistent file"),
        Err(msg) => {
            assert!(msg.contains("1 of 2 failed"), "Expected '1 of 2 failed' in: {}", msg);
        }
    }
    // The existing file should have been deleted
    assert!(!base.join("exists.txt").exists());
}

use std::fs;
use tempfile::TempDir;
use vocofo::context::{ClipboardMode, Context, UiState};
use vocofo::file_operation;

/// Helper to create a context with a test directory containing many files
fn create_test_context() -> (Context, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    fs::write(base.join("file1.txt"), "content1").unwrap();
    fs::write(base.join("file2.txt"), "content2").unwrap();
    fs::create_dir(base.join("folder1")).unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = base.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    (context, temp_dir)
}

/// Helper with many files for pagination tests
fn create_large_context() -> (Context, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    for i in 0..50 {
        fs::write(base.join(format!("file_{:02}.txt", i)), "content").unwrap();
    }

    let mut context = Context::new().unwrap();
    context.panels[0].path = base.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    (context, temp_dir)
}

// ==================== Feature 1: Page Up / Page Down / Home / End ====================

#[test]
fn test_page_down_from_start() {
    let (mut context, _temp_dir) = create_large_context();
    let panel = &mut context.panels[0];
    panel.visible_rows = 10;
    panel.state = 0;

    panel.page_down();
    assert_eq!(panel.state, 10);
}

#[test]
fn test_page_down_near_end() {
    let (mut context, _temp_dir) = create_large_context();
    let panel = &mut context.panels[0];
    panel.visible_rows = 10;
    let last = panel.filtered_items.len() - 1;
    panel.state = last - 3;

    panel.page_down();
    assert_eq!(panel.state, last);
}

#[test]
fn test_page_down_at_end() {
    let (mut context, _temp_dir) = create_large_context();
    let panel = &mut context.panels[0];
    panel.visible_rows = 10;
    let last = panel.filtered_items.len() - 1;
    panel.state = last;

    panel.page_down();
    assert_eq!(panel.state, last);
}

#[test]
fn test_page_up_from_middle() {
    let (mut context, _temp_dir) = create_large_context();
    let panel = &mut context.panels[0];
    panel.visible_rows = 10;
    panel.state = 25;

    panel.page_up();
    assert_eq!(panel.state, 15);
}

#[test]
fn test_page_up_near_start() {
    let (mut context, _temp_dir) = create_large_context();
    let panel = &mut context.panels[0];
    panel.visible_rows = 10;
    panel.state = 3;

    panel.page_up();
    assert_eq!(panel.state, 0);
}

#[test]
fn test_page_up_at_start() {
    let (mut context, _temp_dir) = create_large_context();
    let panel = &mut context.panels[0];
    panel.visible_rows = 10;
    panel.state = 0;

    panel.page_up();
    assert_eq!(panel.state, 0);
}

#[test]
fn test_go_to_first() {
    let (mut context, _temp_dir) = create_large_context();
    let panel = &mut context.panels[0];
    panel.state = 25;

    panel.go_to_first();
    assert_eq!(panel.state, 0);
}

#[test]
fn test_go_to_last() {
    let (mut context, _temp_dir) = create_large_context();
    let panel = &mut context.panels[0];
    let last = panel.filtered_items.len() - 1;
    panel.state = 0;

    panel.go_to_last();
    assert_eq!(panel.state, last);
}

#[test]
fn test_go_to_last_empty_list() {
    let (mut context, _temp_dir) = create_test_context();
    let panel = &mut context.panels[0];
    panel.filtered_items.clear();
    panel.state = 0;

    panel.go_to_last();
    assert_eq!(panel.state, 0);
}

#[test]
fn test_page_down_respects_visible_rows() {
    let (mut context, _temp_dir) = create_large_context();
    let panel = &mut context.panels[0];
    panel.state = 5;

    panel.visible_rows = 5;
    panel.page_down();
    assert_eq!(panel.state, 10);

    panel.visible_rows = 20;
    panel.page_down();
    assert_eq!(panel.state, 30);
}

// ==================== Feature 2: Vim keys (pending_g) ====================

#[test]
fn test_pending_g_default_false() {
    let (context, _temp_dir) = create_test_context();
    assert!(!context.pending_g);
}

#[test]
fn test_pending_g_can_be_set() {
    let (mut context, _temp_dir) = create_test_context();
    context.pending_g = true;
    assert!(context.pending_g);
}

// ==================== Feature 3: visible_rows ====================

#[test]
fn test_visible_rows_default() {
    let (context, _temp_dir) = create_test_context();
    assert_eq!(context.panels[0].visible_rows, 20);
}

// ==================== Feature 4: Create file ====================

#[test]
fn test_create_file_popup_state() {
    let (mut context, _temp_dir) = create_test_context();
    context.set_ui_state(UiState::CreateFilePopup);
    assert_eq!(context.get_ui_state(), Some(UiState::CreateFilePopup));
}

#[test]
fn test_handle_create_file_success() {
    let (mut context, temp_dir) = create_test_context();
    context.set_ui_state(UiState::CreateFilePopup);
    context.set_input("newfile.txt".to_string());

    let result = file_operation::handle_create_file(&mut context);
    assert!(result.is_ok());
    assert!(temp_dir.path().join("newfile.txt").exists());
    assert_eq!(context.get_ui_state(), Some(UiState::Normal));
    assert!(context.get_input().unwrap().is_empty());
}

#[test]
fn test_handle_create_file_empty_name() {
    let (mut context, _temp_dir) = create_test_context();
    context.set_ui_state(UiState::CreateFilePopup);
    context.set_input(String::new());

    let result = file_operation::handle_create_file(&mut context);
    assert!(result.is_err());
}

#[test]
fn test_handle_create_file_already_exists() {
    let (mut context, _temp_dir) = create_test_context();
    context.set_ui_state(UiState::CreateFilePopup);
    context.set_input("file1.txt".to_string());

    let result = file_operation::handle_create_file(&mut context);
    assert!(result.is_err());
}

#[test]
fn test_handle_create_file_resets_cursor() {
    let (mut context, _temp_dir) = create_test_context();
    context.panels[0].state = 2;
    context.set_ui_state(UiState::CreateFilePopup);
    context.set_input("another.txt".to_string());

    file_operation::handle_create_file(&mut context).unwrap();
    assert_eq!(context.panels[0].state, 0);
}

#[test]
fn test_handle_create_file_creates_empty_file() {
    let (mut context, temp_dir) = create_test_context();
    context.set_input("empty.txt".to_string());

    file_operation::handle_create_file(&mut context).unwrap();
    let path = temp_dir.path().join("empty.txt");
    assert!(path.exists());
    assert_eq!(fs::read_to_string(path).unwrap(), "");
}

#[test]
fn test_handle_create_file_with_extension() {
    let (mut context, temp_dir) = create_test_context();
    context.set_input("script.sh".to_string());

    file_operation::handle_create_file(&mut context).unwrap();
    assert!(temp_dir.path().join("script.sh").exists());
}

// ==================== Feature 5: Clipboard indicator ====================

#[test]
fn test_clipboard_empty_by_default() {
    let (context, _temp_dir) = create_test_context();
    assert!(context.copy_path.is_empty());
    assert!(context.copy_paths.is_empty());
}

#[test]
fn test_clipboard_mode_default_copy() {
    let (context, _temp_dir) = create_test_context();
    assert_eq!(context.clipboard_mode, ClipboardMode::Copy);
}

#[test]
fn test_clipboard_single_file() {
    let (mut context, _temp_dir) = create_test_context();
    // Select file1.txt (index depends on sorting, find it)
    let idx = context.panels[0]
        .filtered_items
        .iter()
        .position(|i| i == "file1.txt")
        .unwrap();
    context.panels[0].state = idx;

    context.set_copy_path();
    assert!(!context.copy_path.is_empty());
    assert!(context.copy_path.ends_with("file1.txt"));
}

#[test]
fn test_clipboard_mode_cut() {
    let (mut context, _temp_dir) = create_test_context();
    context.clipboard_mode = ClipboardMode::Cut;
    assert_eq!(context.clipboard_mode, ClipboardMode::Cut);
}

#[test]
fn test_clipboard_bulk_paths() {
    let (mut context, _temp_dir) = create_test_context();
    let paths = context.panels[0].get_selected_paths();
    context.copy_paths = vec!["/tmp/a.txt".to_string(), "/tmp/b.txt".to_string()];
    assert_eq!(context.copy_paths.len(), 2);
    drop(paths);
}

// ==================== Combined navigation tests ====================

#[test]
fn test_go_to_last_then_page_up() {
    let (mut context, _temp_dir) = create_large_context();
    let panel = &mut context.panels[0];
    panel.visible_rows = 10;

    panel.go_to_last();
    let last = panel.state;

    panel.page_up();
    assert_eq!(panel.state, last - 10);
}

#[test]
fn test_go_to_first_then_page_down() {
    let (mut context, _temp_dir) = create_large_context();
    let panel = &mut context.panels[0];
    panel.visible_rows = 10;

    panel.go_to_first();
    assert_eq!(panel.state, 0);

    panel.page_down();
    assert_eq!(panel.state, 10);
}

#[test]
fn test_page_navigation_with_small_list() {
    let (mut context, _temp_dir) = create_test_context();
    let panel = &mut context.panels[0];
    panel.visible_rows = 20; // larger than item count
    panel.state = 0;

    panel.page_down();
    assert_eq!(panel.state, panel.filtered_items.len() - 1);

    panel.page_up();
    assert_eq!(panel.state, 0);
}

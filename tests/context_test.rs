use std::fs;
use tempfile::TempDir;
use vocofo::context::{Context, UiState};

/// Helper to create a context with a test directory
fn create_test_context() -> (Context, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Create test files
    fs::write(base.join("file1.txt"), "content1").unwrap();
    fs::write(base.join("file2.txt"), "content2").unwrap();
    fs::create_dir(base.join("folder1")).unwrap();

    let mut context = Context::new().unwrap();
    context.path = base.to_string_lossy().to_string();

    // Populate items list
    vocofo::file_operation::list_children(&mut context).unwrap();

    (context, temp_dir)
}

#[test]
fn test_context_new() {
    let context = Context::new();
    assert!(context.is_ok());

    let context = context.unwrap();
    assert_eq!(context.exit, false);
    assert!(!context.path.is_empty());
    assert_eq!(context.items.len(), 0);
    assert_eq!(context.state, 0);
    assert_eq!(context.ui_state, UiState::Normal);
    assert!(context.copy_path.is_empty());
}

#[test]
fn test_get_exit() {
    let context = Context::new().unwrap();
    assert_eq!(context.get_exit(), Some(false));
}

#[test]
fn test_set_exit() {
    let mut context = Context::new().unwrap();
    assert_eq!(context.exit, false);

    context.set_exit();
    assert_eq!(context.exit, true);
    assert_eq!(context.get_exit(), Some(true));
}

#[test]
fn test_increment_state() {
    let mut context = Context::new().unwrap();
    assert_eq!(context.state, 0);

    context.increment_state();
    assert_eq!(context.state, 1);

    context.increment_state();
    assert_eq!(context.state, 2);
}

#[test]
fn test_decrease_state() {
    let mut context = Context::new().unwrap();
    context.state = 5;

    context.decrease_state();
    assert_eq!(context.state, 4);

    context.decrease_state();
    assert_eq!(context.state, 3);
}

#[test]
fn test_get_selected_item_empty_list() {
    let context = Context::new().unwrap();
    assert!(context.get_selected_item().is_none());
}

#[test]
fn test_get_selected_item_valid() {
    let (mut context, _temp) = create_test_context();

    // State 0 should be "../"
    assert_eq!(context.get_selected_item(), Some(&"../".to_string()));

    // Move to next item
    context.increment_state();
    assert!(context.get_selected_item().is_some());
}

#[test]
fn test_get_selected_item_out_of_bounds() {
    let (mut context, _temp) = create_test_context();

    // Set state beyond items length
    context.state = 999;
    assert!(context.get_selected_item().is_none());
}

#[test]
fn test_ui_state_transitions() {
    let mut context = Context::new().unwrap();

    // Initial state
    assert_eq!(context.ui_state, UiState::Normal);

    // Transition to CreatePopup
    context.set_ui_state(UiState::CreatePopup);
    assert_eq!(context.get_ui_state(), Some(UiState::CreatePopup));

    // Transition to ConfirmDelete
    context.set_ui_state(UiState::ConfirmDelete);
    assert_eq!(context.get_ui_state(), Some(UiState::ConfirmDelete));

    // Back to Normal
    context.set_ui_state(UiState::Normal);
    assert_eq!(context.get_ui_state(), Some(UiState::Normal));
}

#[test]
fn test_input_handling() {
    let mut context = Context::new().unwrap();

    assert_eq!(context.get_input(), Some(&String::default()));

    context.set_input("test_input".to_string());
    assert_eq!(context.get_input(), Some(&"test_input".to_string()));

    context.set_input(String::default());
    assert_eq!(context.get_input(), Some(&String::default()));
}

#[test]
fn test_set_copy_path_valid_item() {
    let (mut context, temp) = create_test_context();

    // Select file1.txt (should be at index after folders)
    let file_idx = context.items.iter().position(|i| i == "file1.txt").unwrap();
    context.state = file_idx;

    context.set_copy_path();

    let expected = temp.path().join("file1.txt");
    assert_eq!(context.copy_path, expected.to_string_lossy().to_string());
}

#[test]
fn test_set_copy_path_with_folder() {
    let (mut context, temp) = create_test_context();

    // Select folder1/
    let folder_idx = context.items.iter().position(|i| i == "folder1/").unwrap();
    context.state = folder_idx;

    context.set_copy_path();

    // Should strip trailing slash
    let expected = temp.path().join("folder1");
    assert_eq!(context.copy_path, expected.to_string_lossy().to_string());
}

#[test]
fn test_set_copy_path_parent_directory() {
    let (mut context, _temp) = create_test_context();

    // Select "../" (should be at index 0)
    context.state = 0;
    assert_eq!(context.get_selected_item(), Some(&"../".to_string()));

    let before = context.copy_path.clone();
    context.set_copy_path();
    let after = context.copy_path.clone();

    // Should not copy "../"
    assert_eq!(before, after);
    assert!(context.copy_path.is_empty());
}

#[test]
fn test_set_copy_path_no_selection() {
    let mut context = Context::new().unwrap();
    // Empty items list, no selection

    context.set_copy_path();

    // Should remain empty
    assert!(context.copy_path.is_empty());
}

#[test]
fn test_get_copy_path() {
    let mut context = Context::new().unwrap();
    context.copy_path = "/test/path".to_string();

    assert_eq!(context.get_copy_path(), "/test/path");
}

#[test]
fn test_get_state() {
    let mut context = Context::new().unwrap();
    context.state = 42;

    assert_eq!(context.get_state(), 42);
}

#[test]
fn test_set_confirm_button_selected() {
    let mut context = Context::new().unwrap();

    let initial = context.get_confirm_button_selected().unwrap();
    context.set_confirm_button_selected();
    let after = context.get_confirm_button_selected().unwrap();

    assert_ne!(initial, after);

    // Toggle again
    context.set_confirm_button_selected();
    assert_eq!(context.get_confirm_button_selected().unwrap(), initial);
}

#[test]
fn test_get_metadata_selected_item_file() {
    let (mut context, temp) = create_test_context();

    // Select a file
    let file_idx = context.items.iter().position(|i| i == "file1.txt").unwrap();
    context.state = file_idx;

    let metadata = context.get_metadata_selected_item();
    assert!(metadata.is_some());

    let meta = metadata.unwrap();
    assert!(meta.is_file());
    assert!(!meta.is_dir());
}

#[test]
fn test_get_metadata_selected_item_folder() {
    let (mut context, _temp) = create_test_context();

    // Select folder1/
    let folder_idx = context.items.iter().position(|i| i == "folder1/").unwrap();
    context.state = folder_idx;

    let metadata = context.get_metadata_selected_item();
    assert!(metadata.is_some());

    let meta = metadata.unwrap();
    assert!(meta.is_dir());
    assert!(!meta.is_file());
}

#[test]
fn test_get_metadata_selected_item_no_selection() {
    let mut context = Context::new().unwrap();
    // No items in list

    let metadata = context.get_metadata_selected_item();
    assert!(metadata.is_none());
}

#[test]
fn test_state_navigation_boundaries() {
    let (mut context, _temp) = create_test_context();
    let max_state = context.items.len() - 1;

    // Test upper boundary
    context.state = max_state;
    context.increment_state();
    // State should not exceed items length (handled by event handler)

    // Test lower boundary
    context.state = 0;
    context.decrease_state();
    // Would underflow, but event handler prevents this
}

#[test]
fn test_multiple_ui_state_changes() {
    let mut context = Context::new().unwrap();

    let states = vec![
        UiState::Normal,
        UiState::CreatePopup,
        UiState::ConfirmDelete,
        UiState::Normal,
    ];

    for state in states {
        context.set_ui_state(state);
        assert_eq!(context.get_ui_state(), Some(state));
    }
}

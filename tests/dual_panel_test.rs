use std::fs;
use std::sync::Arc;
use tempfile::TempDir;
use vocofo::backend::FilesystemBackend;
use vocofo::background_op;
use vocofo::context::{ClipboardMode, Context};
use vocofo::file_operation;
use vocofo::local_backend::LocalBackend;

/// Helper: create context with two separate temp directories for each panel
fn create_dual_panel_context() -> (Context, TempDir, TempDir) {
    let left_dir = TempDir::new().unwrap();
    let right_dir = TempDir::new().unwrap();

    // Left panel files
    fs::write(left_dir.path().join("left_file.txt"), "left content").unwrap();
    fs::create_dir(left_dir.path().join("left_folder")).unwrap();

    // Right panel files
    fs::write(right_dir.path().join("right_file.txt"), "right content").unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = left_dir.path().to_string_lossy().to_string();
    context.panels[1].path = right_dir.path().to_string_lossy().to_string();

    file_operation::list_children(&mut context.panels[0]).unwrap();
    file_operation::list_children(&mut context.panels[1]).unwrap();

    (context, left_dir, right_dir)
}

// ============================================================================
// Panel Switching Tests
// ============================================================================

#[test]
fn test_toggle_active_panel() {
    let mut context = Context::new().unwrap();
    assert_eq!(context.active_panel, 0);

    context.toggle_active_panel();
    assert_eq!(context.active_panel, 1);

    context.toggle_active_panel();
    assert_eq!(context.active_panel, 0);
}

#[test]
fn test_active_returns_correct_panel() {
    let (mut context, _left, _right) = create_dual_panel_context();

    let left_path = context.panels[0].path.clone();
    let right_path = context.panels[1].path.clone();

    assert_eq!(context.active().path, left_path);

    context.toggle_active_panel();
    assert_eq!(context.active().path, right_path);
}

// ============================================================================
// Independent Navigation Tests
// ============================================================================

#[test]
fn test_panels_have_independent_state() {
    let (mut context, _left, _right) = create_dual_panel_context();

    // Navigate panel 0
    context.panels[0].state = 2;

    // Panel 1 should be unaffected
    assert_eq!(context.panels[1].state, 0);
}

#[test]
fn test_panels_have_independent_items() {
    let (context, _left, _right) = create_dual_panel_context();

    // Left panel should have left_file.txt
    assert!(context.panels[0].items.iter().any(|i| i == "left_file.txt"));
    assert!(
        !context.panels[0]
            .items
            .iter()
            .any(|i| i == "right_file.txt")
    );

    // Right panel should have right_file.txt
    assert!(
        context.panels[1]
            .items
            .iter()
            .any(|i| i == "right_file.txt")
    );
    assert!(!context.panels[1].items.iter().any(|i| i == "left_file.txt"));
}

#[test]
fn test_navigation_affects_only_active_panel() {
    let (mut context, _left, _right) = create_dual_panel_context();

    // Move active panel (0) down
    context.active_mut().increment_state();
    assert_eq!(context.panels[0].state, 1);
    assert_eq!(context.panels[1].state, 0);

    // Switch to panel 1 and move
    context.toggle_active_panel();
    context.active_mut().increment_state();
    assert_eq!(context.panels[0].state, 1);
    assert_eq!(context.panels[1].state, 1);
}

// ============================================================================
// Independent Caching Tests
// ============================================================================

#[test]
fn test_panels_have_independent_cache() {
    let (mut context, _left, _right) = create_dual_panel_context();

    // Invalidate only panel 0
    context.panels[0].invalidate_directory_cache();

    assert!(context.panels[0].items_dirty);
    assert!(!context.panels[1].items_dirty);
}

#[test]
fn test_invalidate_all_caches() {
    let (mut context, _left, _right) = create_dual_panel_context();

    // Both should start clean after list_children
    assert!(!context.panels[0].items_dirty);
    assert!(!context.panels[1].items_dirty);

    context.invalidate_all_caches();

    assert!(context.panels[0].items_dirty);
    assert!(context.panels[1].items_dirty);
}

// ============================================================================
// Navigate to Parent Tests
// ============================================================================

#[test]
fn test_navigate_to_parent() {
    let temp_dir = TempDir::new().unwrap();
    let sub_dir = temp_dir.path().join("subdir");
    fs::create_dir(&sub_dir).unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = sub_dir.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    // Set cursor to a non-../ item
    context.panels[0].state = 0; // on ../

    let result = context.panels[0].navigate_to_parent();
    assert!(result.is_none()); // no error

    // Path should now be the parent
    let expected = fs::canonicalize(temp_dir.path()).unwrap();
    assert_eq!(
        context.panels[0].path,
        expected.to_string_lossy().to_string()
    );
    assert_eq!(context.panels[0].state, 0); // reset to top
}

#[test]
fn test_navigate_to_parent_resets_state() {
    let temp_dir = TempDir::new().unwrap();
    let sub_dir = temp_dir.path().join("child");
    fs::create_dir(&sub_dir).unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = sub_dir.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();
    context.panels[0].state = 5; // some random position

    context.panels[0].navigate_to_parent();

    assert_eq!(context.panels[0].state, 0);
}

#[test]
fn test_navigate_to_parent_invalidates_cache() {
    let temp_dir = TempDir::new().unwrap();
    let sub_dir = temp_dir.path().join("child");
    fs::create_dir(&sub_dir).unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = sub_dir.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    assert!(!context.panels[0].items_dirty);

    context.panels[0].navigate_to_parent();

    assert!(context.panels[0].items_dirty);
}

// ============================================================================
// Cross-Panel Copy/Paste Tests
// ============================================================================

#[test]
fn test_cross_panel_copy_paste() {
    let (mut context, _left, right) = create_dual_panel_context();

    // Select left_file.txt in panel 0
    let file_idx = context.panels[0]
        .items
        .iter()
        .position(|i| i == "left_file.txt")
        .unwrap();
    context.panels[0].state = file_idx;

    // Copy from panel 0
    context.set_copy_path();
    assert!(!context.copy_path.is_empty());
    assert!(context.copy_path.contains("left_file.txt"));

    // Switch to panel 1
    context.toggle_active_panel();
    assert_eq!(context.active_panel, 1);

    // Paste via background op
    let (from, to) = file_operation::resolve_paste_paths(&mut context).unwrap();
    let backend: Arc<dyn FilesystemBackend> = Arc::new(LocalBackend::new());
    let rx = background_op::spawn_copy_with_backend(
        Arc::clone(&backend),
        Arc::clone(&backend),
        from,
        to,
        "test".to_string(),
        None,
    );
    let result = rx.recv().unwrap();
    assert!(result.result.is_ok());

    // File should exist in right panel's directory
    assert!(right.path().join("left_file.txt").exists());
}

#[test]
fn test_cross_panel_cut_move() {
    let (mut context, left, right) = create_dual_panel_context();

    // Select left_file.txt in panel 0
    let file_idx = context.panels[0]
        .items
        .iter()
        .position(|i| i == "left_file.txt")
        .unwrap();
    context.panels[0].state = file_idx;

    // Cut from panel 0
    context.set_copy_path();
    context.clipboard_mode = ClipboardMode::Cut;

    // Switch to panel 1
    context.toggle_active_panel();

    // Move via background op (copy + delete)
    let (from, to) = file_operation::resolve_paste_paths(&mut context).unwrap();
    let backend: Arc<dyn FilesystemBackend> = Arc::new(LocalBackend::new());
    let rx = background_op::spawn_move_with_backend(
        Arc::clone(&backend),
        Arc::clone(&backend),
        from,
        to,
        "test".to_string(),
        None,
    );
    let result = rx.recv().unwrap();
    assert!(result.result.is_ok());

    // File should exist in right, not in left
    assert!(right.path().join("left_file.txt").exists());
    assert!(!left.path().join("left_file.txt").exists());
}

// ============================================================================
// Preview Toggle Tests
// ============================================================================

#[test]
fn test_show_preview_default_off() {
    let context = Context::new().unwrap();
    assert!(!context.show_preview);
}

#[test]
fn test_show_preview_toggle() {
    let mut context = Context::new().unwrap();

    context.show_preview = !context.show_preview;
    assert!(context.show_preview);

    context.show_preview = !context.show_preview;
    assert!(!context.show_preview);
}

// ============================================================================
// PanelState Construction Tests
// ============================================================================

#[test]
fn test_panel_state_new() {
    let panel =
        vocofo::context::PanelState::new("/test/path".to_string(), Arc::new(LocalBackend::new()));
    assert_eq!(panel.path, "/test/path");
    assert!(panel.items.is_empty());
    assert_eq!(panel.state, 0);
    assert!(panel.items_dirty);
    assert!(panel.preview_content.is_none());
    assert!(panel.preview_last_item.is_none());
}

#[test]
fn test_context_initializes_two_panels() {
    let context = Context::new().unwrap();
    assert_eq!(context.panels[0].path, context.panels[1].path);
    assert_eq!(context.active_panel, 0);
}

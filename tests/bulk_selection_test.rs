use std::fs;
use std::sync::Arc;
use tempfile::TempDir;
use vocofo::backend::FilesystemBackend;
use vocofo::background_op;
use vocofo::context::Context;
use vocofo::file_operation;
use vocofo::local_backend::LocalBackend;

fn create_bulk_context() -> (Context, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    fs::write(base.join("file1.txt"), "content1").unwrap();
    fs::write(base.join("file2.txt"), "content2").unwrap();
    fs::write(base.join("file3.txt"), "content3").unwrap();
    fs::create_dir(base.join("folder1")).unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = base.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    (context, temp_dir)
}

// ============================================================================
// Toggle Selection
// ============================================================================

#[test]
fn test_toggle_selection_select() {
    let (mut context, _temp) = create_bulk_context();

    let idx = context.panels[0].filtered_items.iter().position(|i| i == "file1.txt").unwrap();
    context.panels[0].state = idx;
    context.panels[0].toggle_selection();

    assert!(context.panels[0].selected.contains("file1.txt"));
}

#[test]
fn test_toggle_selection_deselect() {
    let (mut context, _temp) = create_bulk_context();

    let idx = context.panels[0].filtered_items.iter().position(|i| i == "file1.txt").unwrap();
    context.panels[0].state = idx;

    context.panels[0].toggle_selection();
    assert!(context.panels[0].selected.contains("file1.txt"));

    context.panels[0].toggle_selection();
    assert!(!context.panels[0].selected.contains("file1.txt"));
}

#[test]
fn test_toggle_selection_ignores_parent() {
    let (mut context, _temp) = create_bulk_context();

    context.panels[0].state = 0; // ../
    context.panels[0].toggle_selection();

    assert!(!context.panels[0].has_selection());
}

#[test]
fn test_toggle_selection_multiple_items() {
    let (mut context, _temp) = create_bulk_context();

    let idx1 = context.panels[0].filtered_items.iter().position(|i| i == "file1.txt").unwrap();
    let idx2 = context.panels[0].filtered_items.iter().position(|i| i == "file2.txt").unwrap();

    context.panels[0].state = idx1;
    context.panels[0].toggle_selection();
    context.panels[0].state = idx2;
    context.panels[0].toggle_selection();

    assert_eq!(context.panels[0].selected.len(), 2);
    assert!(context.panels[0].selected.contains("file1.txt"));
    assert!(context.panels[0].selected.contains("file2.txt"));
}

// ============================================================================
// Select All / Clear
// ============================================================================

#[test]
fn test_select_all() {
    let (mut context, _temp) = create_bulk_context();

    context.panels[0].select_all();

    // Everything except ../ should be selected
    assert!(!context.panels[0].selected.contains("../"));
    assert!(context.panels[0].selected.contains("file1.txt"));
    assert!(context.panels[0].selected.contains("file2.txt"));
    assert!(context.panels[0].selected.contains("file3.txt"));
    assert!(context.panels[0].selected.contains("folder1/"));
}

#[test]
fn test_clear_selection() {
    let (mut context, _temp) = create_bulk_context();

    context.panels[0].select_all();
    assert!(context.panels[0].has_selection());

    context.panels[0].clear_selection();
    assert!(!context.panels[0].has_selection());
    assert!(context.panels[0].selected.is_empty());
}

#[test]
fn test_has_selection() {
    let (mut context, _temp) = create_bulk_context();

    assert!(!context.panels[0].has_selection());

    let idx = context.panels[0].filtered_items.iter().position(|i| i == "file1.txt").unwrap();
    context.panels[0].state = idx;
    context.panels[0].toggle_selection();

    assert!(context.panels[0].has_selection());
}

// ============================================================================
// Get Selected Paths
// ============================================================================

#[test]
fn test_get_selected_paths() {
    let (mut context, temp) = create_bulk_context();

    let idx = context.panels[0].filtered_items.iter().position(|i| i == "file1.txt").unwrap();
    context.panels[0].state = idx;
    context.panels[0].toggle_selection();

    let paths = context.panels[0].get_selected_paths();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0], temp.path().join("file1.txt").to_string_lossy().to_string());
}

#[test]
fn test_get_selected_paths_strips_trailing_slash() {
    let (mut context, temp) = create_bulk_context();

    let idx = context.panels[0].filtered_items.iter().position(|i| i == "folder1/").unwrap();
    context.panels[0].state = idx;
    context.panels[0].toggle_selection();

    let paths = context.panels[0].get_selected_paths();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0], temp.path().join("folder1").to_string_lossy().to_string());
}

// ============================================================================
// Batch Delete
// ============================================================================

#[test]
fn test_batch_delete() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    fs::write(base.join("a.txt"), "a").unwrap();
    fs::write(base.join("b.txt"), "b").unwrap();
    fs::write(base.join("c.txt"), "c").unwrap();

    let paths = vec![
        base.join("a.txt"),
        base.join("b.txt"),
    ];

    let backend: Arc<dyn FilesystemBackend> = Arc::new(LocalBackend::new());
    let paths_str: Vec<String> = paths.iter().map(|p| p.to_string_lossy().to_string()).collect();
    let rx = background_op::spawn_delete_batch_with_backend(backend, paths_str, "test delete".to_string());
    let result = rx.recv().unwrap();
    assert!(result.result.is_ok());

    assert!(!base.join("a.txt").exists());
    assert!(!base.join("b.txt").exists());
    assert!(base.join("c.txt").exists()); // untouched
}

// ============================================================================
// Batch Copy
// ============================================================================

#[test]
fn test_batch_copy() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    let src = base.join("src");
    let dst = base.join("dst");
    fs::create_dir(&src).unwrap();
    fs::create_dir(&dst).unwrap();

    fs::write(src.join("a.txt"), "aaa").unwrap();
    fs::write(src.join("b.txt"), "bbb").unwrap();

    let items = vec![
        (src.join("a.txt"), dst.join("a.txt")),
        (src.join("b.txt"), dst.join("b.txt")),
    ];

    let backend: Arc<dyn FilesystemBackend> = Arc::new(LocalBackend::new());
    let items_str: Vec<(String, String)> = items.iter()
        .map(|(f, t)| (f.to_string_lossy().to_string(), t.to_string_lossy().to_string()))
        .collect();
    let rx = background_op::spawn_copy_batch_with_backend(
        Arc::clone(&backend), Arc::clone(&backend),
        items_str, "test copy".to_string(), false, None,
    );
    let result = rx.recv().unwrap();
    assert!(result.result.is_ok());
    assert!(!result.clear_clipboard);

    assert!(dst.join("a.txt").exists());
    assert!(dst.join("b.txt").exists());
    // Source still exists (copy, not move)
    assert!(src.join("a.txt").exists());
}

#[test]
fn test_batch_move() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    let src = base.join("src");
    let dst = base.join("dst");
    fs::create_dir(&src).unwrap();
    fs::create_dir(&dst).unwrap();

    fs::write(src.join("a.txt"), "aaa").unwrap();

    let items = vec![
        (src.join("a.txt"), dst.join("a.txt")),
    ];

    let backend: Arc<dyn FilesystemBackend> = Arc::new(LocalBackend::new());
    let items_str: Vec<(String, String)> = items.iter()
        .map(|(f, t)| (f.to_string_lossy().to_string(), t.to_string_lossy().to_string()))
        .collect();
    let rx = background_op::spawn_copy_batch_with_backend(
        Arc::clone(&backend), Arc::clone(&backend),
        items_str, "test move".to_string(), true, None,
    );
    let result = rx.recv().unwrap();
    assert!(result.result.is_ok());
    assert!(result.clear_clipboard);

    assert!(dst.join("a.txt").exists());
    assert!(!src.join("a.txt").exists()); // source deleted
}

// ============================================================================
// Multi-select Copy Paths
// ============================================================================

#[test]
fn test_copy_paths_from_selection() {
    let (mut context, _temp) = create_bulk_context();

    context.panels[0].select_all();
    context.copy_paths = context.panels[0].get_selected_paths();

    assert!(context.copy_paths.len() >= 4); // folder1, file1, file2, file3
}

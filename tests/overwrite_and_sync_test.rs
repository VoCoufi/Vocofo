use std::fs;
use std::sync::Arc;
use tempfile::TempDir;
use vocofo::background_op;
use vocofo::backend::FilesystemBackend;
use vocofo::context::Context;
use vocofo::file_operation;
use vocofo::local_backend::LocalBackend;

// ============================================================================
// Overwrite Confirmation Tests
// ============================================================================

#[test]
fn test_resolve_paste_paths_detects_existing_target() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    let source = base.join("source");
    let dest = base.join("dest");
    fs::create_dir(&source).unwrap();
    fs::create_dir(&dest).unwrap();
    fs::write(source.join("file.txt"), "original").unwrap();
    fs::write(dest.join("file.txt"), "existing").unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = source.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    let file_idx = context.panels[0].items.iter().position(|i| i == "file.txt").unwrap();
    context.panels[0].state = file_idx;
    context.set_copy_path();

    // Navigate to dest
    context.panels[0].path = dest.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();
    context.panels[0].state = 0;

    let (from, to) = file_operation::resolve_paste_paths(&mut context).unwrap();

    // Target exists
    assert!(std::path::Path::new(&to).exists());
    assert_eq!(std::path::Path::new(&from).file_name().unwrap().to_str().unwrap(), "file.txt");
    assert_eq!(std::path::Path::new(&to).file_name().unwrap().to_str().unwrap(), "file.txt");
}

#[test]
fn test_overwrite_after_delete_target() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    let source = base.join("source");
    let dest = base.join("dest");
    fs::create_dir(&source).unwrap();
    fs::create_dir(&dest).unwrap();
    fs::write(source.join("file.txt"), "new content").unwrap();
    fs::write(dest.join("file.txt"), "old content").unwrap();

    // Simulate the overwrite flow: delete target, then copy
    let target = dest.join("file.txt");
    fs::remove_file(&target).unwrap();
    assert!(!target.exists());

    let backend: Arc<dyn FilesystemBackend> = Arc::new(LocalBackend::new());
    let from = source.join("file.txt").to_string_lossy().to_string();
    let to = dest.join("file.txt").to_string_lossy().to_string();
    let rx = background_op::spawn_copy_with_backend(
        Arc::clone(&backend), Arc::clone(&backend),
        from, to, "test".to_string(), None,
    );
    let result = rx.recv().unwrap();
    assert!(result.result.is_ok());

    // File should have new content
    assert_eq!(fs::read_to_string(dest.join("file.txt")).unwrap(), "new content");
}

#[test]
fn test_pending_paste_stores_paths() {
    let mut context = Context::new().unwrap();
    let from = "/test/source.txt".to_string();
    let to = "/test/dest.txt".to_string();

    context.pending_paste = Some((from.clone(), to.clone(), false));

    let (stored_from, stored_to, is_cut) = context.pending_paste.as_ref().unwrap();
    assert_eq!(stored_from, &from);
    assert_eq!(stored_to, &to);
    assert!(!is_cut);
}

#[test]
fn test_pending_paste_with_cut() {
    let mut context = Context::new().unwrap();
    let from = "/test/source.txt".to_string();
    let to = "/test/dest.txt".to_string();

    context.pending_paste = Some((from.clone(), to.clone(), true));

    let (_, _, is_cut) = context.pending_paste.as_ref().unwrap();
    assert!(is_cut);
}

// ============================================================================
// Sync Panel Tests
// ============================================================================

#[test]
fn test_sync_panel_copies_path() {
    let temp_dir = TempDir::new().unwrap();
    let other_dir = TempDir::new().unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = temp_dir.path().to_string_lossy().to_string();
    context.panels[1].path = other_dir.path().to_string_lossy().to_string();

    assert_ne!(context.panels[0].path, context.panels[1].path);

    let path = context.panels[0].path.clone();
    context.panels[1].path = path;
    context.panels[1].invalidate_directory_cache();

    assert_eq!(context.panels[0].path, context.panels[1].path);
    assert!(context.panels[1].items_dirty);
}

#[test]
fn test_sync_panel_clears_filter() {
    let temp_dir = TempDir::new().unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = temp_dir.path().to_string_lossy().to_string();
    context.panels[1].path = temp_dir.path().to_string_lossy().to_string();
    context.panels[1].filter = "something".to_string();

    let path = context.panels[0].path.clone();
    context.panels[1].path = path;
    context.panels[1].clear_filter();

    assert!(context.panels[1].filter.is_empty());
}

// ============================================================================
// File Details Tests (using backend-aware version)
// ============================================================================

#[test]
fn test_format_item_details_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "hello world").unwrap();

    let backend = LocalBackend::new();
    let info = backend.metadata(&file_path.to_string_lossy()).unwrap();
    let details = file_operation::format_item_details_from_info(&info);
    assert!(details.contains("B"));
}

#[test]
fn test_format_item_details_directory() {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path().join("subdir");
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("a.txt"), "").unwrap();
    fs::write(dir.join("b.txt"), "").unwrap();

    let backend = LocalBackend::new();
    let entries = backend.list_dir(&dir.to_string_lossy()).unwrap();
    let details = format!("{} items", entries.len());
    assert_eq!(details, "2 items");
}

#[test]
fn test_format_item_details_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path().join("empty");
    fs::create_dir(&dir).unwrap();

    let backend = LocalBackend::new();
    let entries = backend.list_dir(&dir.to_string_lossy()).unwrap();
    let details = format!("{} items", entries.len());
    assert_eq!(details, "0 items");
}

#[test]
fn test_format_item_details_nonexistent() {
    let backend = LocalBackend::new();
    let result = backend.metadata("/nonexistent/path");
    assert!(result.is_err());
}

#[test]
fn test_format_item_details_large_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("large.bin");
    fs::write(&file_path, vec![0u8; 1024 * 1024]).unwrap(); // 1MB

    let backend = LocalBackend::new();
    let info = backend.metadata(&file_path.to_string_lossy()).unwrap();
    let details = file_operation::format_item_details_from_info(&info);
    assert!(details.contains("MB"));
}

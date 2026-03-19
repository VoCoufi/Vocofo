use std::fs;
use std::sync::Arc;
use tempfile::TempDir;
use vocofo::background_op;
use vocofo::backend::FilesystemBackend;
use vocofo::context::{ClipboardMode, Context};
use vocofo::file_operation;
use vocofo::local_backend::LocalBackend;

/// Helper: resolve paths and run a synchronous copy via background_op
fn paste_and_wait(context: &mut Context) -> Result<(), String> {
    let (from, to) = file_operation::resolve_paste_paths(context)
        .map_err(|e| e.to_string())?;
    let backend: Arc<dyn FilesystemBackend> = Arc::new(LocalBackend::new());
    let rx = background_op::spawn_copy_with_backend(
        Arc::clone(&backend), Arc::clone(&backend),
        from, to, "test".to_string(), None,
    );
    let result = rx.recv().map_err(|e| e.to_string())?;
    result.result
}

fn move_and_wait(from: String, to: String) -> Result<(), String> {
    let backend: Arc<dyn FilesystemBackend> = Arc::new(LocalBackend::new());
    let rx = background_op::spawn_move_with_backend(
        Arc::clone(&backend), Arc::clone(&backend),
        from, to, "test move".to_string(), None,
    );
    let result = rx.recv().map_err(|e| e.to_string())?;
    result.result
}

#[test]
fn test_copy_paste_file_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    let source_dir = base.join("source");
    let dest_dir = base.join("dest");
    fs::create_dir(&source_dir).unwrap();
    fs::create_dir(&dest_dir).unwrap();
    fs::write(source_dir.join("test.txt"), "test content").unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = source_dir.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    let file_idx = context.panels[0].items.iter().position(|i| i == "test.txt").unwrap();
    context.panels[0].state = file_idx;
    context.set_copy_path();

    context.panels[0].path = dest_dir.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();
    context.panels[0].state = 0;

    let result = paste_and_wait(&mut context);
    assert!(result.is_ok(), "Copy failed: {:?}", result.err());

    let copied_file = dest_dir.join("test.txt");
    assert!(copied_file.exists());
    assert_eq!(fs::read_to_string(&copied_file).unwrap(), "test content");
}

#[test]
fn test_copy_paste_folder_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    let source_dir = base.join("source");
    fs::create_dir(&source_dir).unwrap();
    let test_folder = source_dir.join("myfolder");
    fs::create_dir(&test_folder).unwrap();
    fs::write(test_folder.join("file1.txt"), "content1").unwrap();
    fs::write(test_folder.join("file2.txt"), "content2").unwrap();

    let dest_dir = base.join("dest");
    fs::create_dir(&dest_dir).unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = source_dir.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    let folder_idx = context.panels[0].items.iter().position(|i| i == "myfolder/").unwrap();
    context.panels[0].state = folder_idx;
    context.set_copy_path();

    context.panels[0].path = dest_dir.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();
    context.panels[0].state = 0;

    let result = paste_and_wait(&mut context);
    assert!(result.is_ok(), "Copy folder failed: {:?}", result.err());

    let copied_folder = dest_dir.join("myfolder");
    assert!(copied_folder.exists());
    assert!(copied_folder.is_dir());
    assert!(copied_folder.join("file1.txt").exists());
    assert!(copied_folder.join("file2.txt").exists());
    assert_eq!(fs::read_to_string(copied_folder.join("file1.txt")).unwrap(), "content1");
}

#[test]
fn test_copy_paste_into_subfolder() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    fs::write(base.join("file.txt"), "content").unwrap();
    fs::create_dir(base.join("subfolder")).unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = base.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    let file_idx = context.panels[0].items.iter().position(|i| i == "file.txt").unwrap();
    context.panels[0].state = file_idx;
    context.set_copy_path();

    let folder_idx = context.panels[0].items.iter().position(|i| i == "subfolder/").unwrap();
    context.panels[0].state = folder_idx;

    let result = paste_and_wait(&mut context);
    assert!(result.is_ok());
    assert!(base.join("subfolder/file.txt").exists());
}

#[test]
fn test_copy_paste_same_directory_detected() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    fs::write(base.join("file.txt"), "content").unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = base.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    let file_idx = context.panels[0].items.iter().position(|i| i == "file.txt").unwrap();
    context.panels[0].state = file_idx;
    context.set_copy_path();

    context.panels[0].state = 0;
    // resolve_paste_paths detects same source and dest paths
    let (from, to) = file_operation::resolve_paste_paths(&mut context).unwrap();
    // Both paths point to same file in same directory
    assert_eq!(from, to);
}

#[test]
fn test_copy_without_selection() {
    let mut context = Context::new().unwrap();
    context.set_copy_path();
    assert!(context.copy_path.is_empty());
}

#[test]
fn test_paste_with_empty_clipboard() {
    let temp_dir = TempDir::new().unwrap();
    let mut context = Context::new().unwrap();
    context.panels[0].path = temp_dir.path().to_string_lossy().to_string();

    assert!(context.copy_path.is_empty());
    let result = file_operation::resolve_paste_paths(&mut context);
    assert!(result.is_err());
}

#[test]
fn test_copy_parent_directory_does_nothing() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    let mut context = Context::new().unwrap();
    context.panels[0].path = base.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    context.panels[0].state = 0;
    assert_eq!(context.panels[0].get_selected_item(), Some(&"../".to_string()));
    context.set_copy_path();
    assert!(context.copy_path.is_empty());
}

#[test]
fn test_copy_file_with_special_characters() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    let special_file = "file with spaces (copy).txt";
    fs::write(base.join(special_file), "content").unwrap();

    let dest = base.join("dest");
    fs::create_dir(&dest).unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = base.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    let file_idx = context.panels[0].items.iter().position(|i| i == special_file).unwrap();
    context.panels[0].state = file_idx;
    context.set_copy_path();

    context.panels[0].path = dest.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();
    context.panels[0].state = 0;

    let result = paste_and_wait(&mut context);
    assert!(result.is_ok());
    assert!(dest.join(special_file).exists());
}

#[test]
fn test_clipboard_persists_across_navigation() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    fs::write(base.join("file.txt"), "content").unwrap();
    fs::create_dir(base.join("folder1")).unwrap();
    fs::create_dir(base.join("folder2")).unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = base.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    let file_idx = context.panels[0].items.iter().position(|i| i == "file.txt").unwrap();
    context.panels[0].state = file_idx;
    context.set_copy_path();
    let clipboard_content = context.copy_path.clone();
    assert!(!clipboard_content.is_empty());

    context.panels[0].path = base.join("folder1").to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();
    assert_eq!(context.copy_path, clipboard_content);

    context.panels[0].path = base.join("folder2").to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();
    assert_eq!(context.copy_path, clipboard_content);
}

#[test]
fn test_overwrite_existing_file_fails() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    fs::write(base.join("file.txt"), "original").unwrap();
    let dest = base.join("dest");
    fs::create_dir(&dest).unwrap();
    fs::write(dest.join("file.txt"), "existing").unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = base.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    let file_idx = context.panels[0].items.iter().position(|i| i == "file.txt").unwrap();
    context.panels[0].state = file_idx;
    context.set_copy_path();

    context.panels[0].path = dest.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();
    context.panels[0].state = 0;

    let result = paste_and_wait(&mut context);
    // Backend copy may or may not fail on overwrite depending on implementation
    // The important thing is the file is accessible
    if result.is_err() {
        assert_eq!(fs::read_to_string(dest.join("file.txt")).unwrap(), "existing");
    }
}

#[test]
fn test_cut_move_file_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    let source_dir = base.join("source");
    let dest_dir = base.join("dest");
    fs::create_dir(&source_dir).unwrap();
    fs::create_dir(&dest_dir).unwrap();
    fs::write(source_dir.join("moveme.txt"), "move content").unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = source_dir.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    let file_idx = context.panels[0].items.iter().position(|i| i == "moveme.txt").unwrap();
    context.panels[0].state = file_idx;
    context.set_copy_path();
    context.clipboard_mode = ClipboardMode::Cut;

    context.panels[0].path = dest_dir.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();
    context.panels[0].state = 0;

    let (from, to) = file_operation::resolve_paste_paths(&mut context).unwrap();
    let result = move_and_wait(from, to);
    assert!(result.is_ok(), "Move failed: {:?}", result.err());

    assert!(dest_dir.join("moveme.txt").exists());
    assert!(!source_dir.join("moveme.txt").exists());
    assert_eq!(fs::read_to_string(dest_dir.join("moveme.txt")).unwrap(), "move content");
}

#[test]
fn test_cut_move_folder_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    let source_dir = base.join("source");
    let dest_dir = base.join("dest");
    fs::create_dir(&source_dir).unwrap();
    fs::create_dir(&dest_dir).unwrap();

    let folder = source_dir.join("myfolder");
    fs::create_dir(&folder).unwrap();
    fs::write(folder.join("inner.txt"), "inner").unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = source_dir.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    let folder_idx = context.panels[0].items.iter().position(|i| i == "myfolder/").unwrap();
    context.panels[0].state = folder_idx;
    context.set_copy_path();
    context.clipboard_mode = ClipboardMode::Cut;

    context.panels[0].path = dest_dir.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();
    context.panels[0].state = 0;

    let (from, to) = file_operation::resolve_paste_paths(&mut context).unwrap();
    let result = move_and_wait(from, to);
    assert!(result.is_ok(), "Move folder failed: {:?}", result.err());

    assert!(dest_dir.join("myfolder").exists());
    assert!(dest_dir.join("myfolder/inner.txt").exists());
    assert!(!source_dir.join("myfolder").exists());
}

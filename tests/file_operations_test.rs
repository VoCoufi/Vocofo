use std::fs;
use std::sync::Arc;
use tempfile::TempDir;
use vocofo::backend::FilesystemBackend;
use vocofo::context::Context;
use vocofo::file_operation::{self, format_size, generate_preview_with_backend, list_children};
use vocofo::local_backend::LocalBackend;

/// Helper function to create a test directory structure
fn setup_test_dir() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let base = temp_dir.path();

    fs::write(base.join("file1.txt"), "content1").unwrap();
    fs::write(base.join("file2.txt"), "content2").unwrap();
    fs::create_dir(base.join("folder1")).unwrap();
    fs::write(base.join("folder1/nested_file.txt"), "nested").unwrap();
    fs::create_dir(base.join("folder1/subfolder")).unwrap();
    fs::create_dir(base.join("folder2")).unwrap();

    temp_dir
}

fn backend() -> Arc<dyn FilesystemBackend> {
    Arc::new(LocalBackend::new())
}

// ============================================================================
// Directory Operations (via backend)
// ============================================================================

#[test]
fn test_create_directory() {
    let temp_dir = TempDir::new().unwrap();
    let new_dir = temp_dir.path().join("new_folder");
    let b = LocalBackend::new();

    assert!(b.create_dir(&new_dir.to_string_lossy()).is_ok());
    assert!(new_dir.exists());
    assert!(new_dir.is_dir());
}

#[test]
fn test_create_nested_directory() {
    let temp_dir = TempDir::new().unwrap();
    let nested_dir = temp_dir.path().join("parent/child/grandchild");
    let b = LocalBackend::new();

    assert!(b.create_dir(&nested_dir.to_string_lossy()).is_ok());
    assert!(nested_dir.exists());
}

#[test]
fn test_create_existing_directory() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path().join("existing");
    fs::create_dir(&dir_path).unwrap();
    let b = LocalBackend::new();

    assert!(b.create_dir(&dir_path.to_string_lossy()).is_ok());
}

// ============================================================================
// Delete Operations (via backend)
// ============================================================================

#[test]
fn test_delete_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("delete_me.txt");
    fs::write(&file_path, "content").unwrap();

    let b = backend();
    assert!(file_operation::delete_with_backend(&b, &file_path.to_string_lossy()).is_ok());
    assert!(!file_path.exists());
}

#[test]
fn test_delete_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path().join("empty_dir");
    fs::create_dir(&dir_path).unwrap();

    let b = backend();
    assert!(file_operation::delete_with_backend(&b, &dir_path.to_string_lossy()).is_ok());
    assert!(!dir_path.exists());
}

#[test]
fn test_delete_directory_with_contents() {
    let temp_dir = setup_test_dir();
    let folder_path = temp_dir.path().join("folder1");

    let b = backend();
    assert!(file_operation::delete_with_backend(&b, &folder_path.to_string_lossy()).is_ok());
    assert!(!folder_path.exists());
}

#[test]
fn test_delete_prevents_parent_directory() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("file.txt"), "test").unwrap();

    let dangerous_path = temp_dir.path().join("../should_not_delete");
    let b = backend();
    let result = file_operation::delete_with_backend(&b, &dangerous_path.to_string_lossy());
    assert!(result.is_err());
}

#[test]
fn test_delete_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent = temp_dir.path().join("does_not_exist.txt");

    let b = backend();
    assert!(file_operation::delete_with_backend(&b, &nonexistent.to_string_lossy()).is_err());
}

// ============================================================================
// Copy Operations (via backend)
// ============================================================================

#[test]
fn test_copy_directory_basic() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source");
    let dest = temp_dir.path().join("destination");

    fs::create_dir(&source).unwrap();
    fs::write(source.join("file.txt"), "content").unwrap();

    let b = LocalBackend::new();
    assert!(
        b.copy_dir(&source.to_string_lossy(), &dest.to_string_lossy())
            .is_ok()
    );

    assert!(dest.exists());
    assert!(dest.join("file.txt").exists());
    assert_eq!(
        fs::read_to_string(dest.join("file.txt")).unwrap(),
        "content"
    );
}

#[test]
fn test_copy_directory_recursive() {
    let temp_dir = setup_test_dir();
    let source = temp_dir.path().join("folder1");
    let dest = temp_dir.path().join("folder1_copy");

    let b = LocalBackend::new();
    assert!(
        b.copy_dir(&source.to_string_lossy(), &dest.to_string_lossy())
            .is_ok()
    );

    assert!(dest.exists());
    assert!(dest.join("nested_file.txt").exists());
    assert!(dest.join("subfolder").exists());
}

// ============================================================================
// List Children Tests
// ============================================================================

#[test]
fn test_list_children_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let mut context = Context::new().unwrap();
    context.panels[0].path = temp_dir.path().to_string_lossy().to_string();

    assert!(list_children(&mut context.panels[0]).is_ok());
    assert_eq!(context.panels[0].items.len(), 1);
    assert_eq!(context.panels[0].items[0], "../");
}

#[test]
fn test_list_children_with_files_and_folders() {
    let temp_dir = setup_test_dir();
    let mut context = Context::new().unwrap();
    context.panels[0].path = temp_dir.path().to_string_lossy().to_string();

    assert!(list_children(&mut context.panels[0]).is_ok());
    assert_eq!(context.panels[0].items.len(), 5);
    assert!(context.panels[0].items[0] == "../");
    assert!(context.panels[0].items[1].ends_with('/'));
    assert!(context.panels[0].items[2].ends_with('/'));
    assert!(!context.panels[0].items[3].ends_with('/'));
    assert!(!context.panels[0].items[4].ends_with('/'));
}

#[test]
fn test_list_children_sorted() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    fs::write(base.join("zebra.txt"), "").unwrap();
    fs::write(base.join("apple.txt"), "").unwrap();
    fs::create_dir(base.join("zoo")).unwrap();
    fs::create_dir(base.join("archive")).unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = base.to_string_lossy().to_string();

    assert!(list_children(&mut context.panels[0]).is_ok());
    assert_eq!(context.panels[0].items[0], "../");
    assert_eq!(context.panels[0].items[1], "archive/");
    assert_eq!(context.panels[0].items[2], "zoo/");
    assert_eq!(context.panels[0].items[3], "apple.txt");
    assert_eq!(context.panels[0].items[4], "zebra.txt");
}

#[test]
fn test_list_children_invalid_path() {
    let mut context = Context::new().unwrap();
    context.panels[0].path = "/nonexistent/path/that/does/not/exist".to_string();
    assert!(list_children(&mut context.panels[0]).is_err());
}

#[test]
fn test_list_children_special_characters() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    fs::write(base.join("file with spaces.txt"), "").unwrap();
    fs::write(base.join("file-with-dashes.txt"), "").unwrap();
    fs::create_dir(base.join("folder (copy)")).unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = base.to_string_lossy().to_string();

    assert!(list_children(&mut context.panels[0]).is_ok());
    assert!(
        context.panels[0]
            .items
            .iter()
            .any(|i| i == "file with spaces.txt")
    );
    assert!(
        context.panels[0]
            .items
            .iter()
            .any(|i| i == "file-with-dashes.txt")
    );
    assert!(
        context.panels[0]
            .items
            .iter()
            .any(|i| i == "folder (copy)/")
    );
}

// ============================================================================
// Format Size Tests
// ============================================================================

#[test]
fn test_format_size_bytes() {
    assert_eq!(format_size(0), "0 B");
    assert_eq!(format_size(100), "100 B");
    assert_eq!(format_size(1023), "1023 B");
}

#[test]
fn test_format_size_kilobytes() {
    assert_eq!(format_size(1024), "1.00 KB");
    assert_eq!(format_size(2048), "2.00 KB");
    assert_eq!(format_size(1536), "1.50 KB");
}

#[test]
fn test_format_size_megabytes() {
    assert_eq!(format_size(1024 * 1024), "1.00 MB");
    assert_eq!(format_size(5 * 1024 * 1024), "5.00 MB");
    assert_eq!(format_size(1536 * 1024), "1.50 MB");
}

#[test]
fn test_format_size_gigabytes() {
    assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
    assert_eq!(format_size(3 * 1024 * 1024 * 1024), "3.00 GB");
}

// ============================================================================
// Preview Tests (via backend)
// ============================================================================

#[test]
fn test_generate_preview_text_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "Hello, World!").unwrap();

    let b = backend();
    let preview = generate_preview_with_backend(&b, &file_path.to_string_lossy());

    assert!(preview.contains("Type: File"));
    assert!(preview.contains("Preview"));
    assert!(preview.contains("Hello, World!"));
}

#[test]
fn test_generate_preview_binary_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("binary.bin");
    fs::write(&file_path, vec![0xFF, 0xFE, 0xFD]).unwrap();

    let b = backend();
    let preview = generate_preview_with_backend(&b, &file_path.to_string_lossy());

    assert!(preview.contains("Type: File"));
    assert!(preview.contains("Binary file"));
}

#[test]
fn test_generate_preview_directory() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();
    fs::write(base.join("file1.txt"), "").unwrap();
    fs::write(base.join("file2.txt"), "").unwrap();

    let b = backend();
    let preview = generate_preview_with_backend(&b, &base.to_string_lossy());

    assert!(preview.contains("Type: Directory"));
    assert!(preview.contains("Contents"));
    assert!(preview.contains("file1.txt"));
    assert!(preview.contains("file2.txt"));
}

#[test]
fn test_generate_preview_empty_directory() {
    let temp_dir = TempDir::new().unwrap();

    let b = backend();
    let preview = generate_preview_with_backend(&b, &temp_dir.path().to_string_lossy());

    assert!(preview.contains("Type: Directory"));
    assert!(preview.contains("Contents"));
}

#[test]
fn test_generate_preview_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent = temp_dir.path().join("nonexistent.txt");

    let b = backend();
    let preview = generate_preview_with_backend(&b, &nonexistent.to_string_lossy());

    assert!(preview.contains("Error"));
}

#[test]
fn test_backend_read_file_preview() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    let content = "Hello, World!\nThis is a test file.";
    fs::write(&file_path, content).unwrap();

    let b = LocalBackend::new();
    let data = b
        .read_file(&file_path.to_string_lossy(), 64 * 1024)
        .unwrap();
    let text = String::from_utf8(data).unwrap();
    assert_eq!(text, content);
}

#[test]
fn test_backend_read_file_large_truncated() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("large.txt");
    let large_content = "A".repeat(70 * 1024);
    fs::write(&file_path, &large_content).unwrap();

    let b = LocalBackend::new();
    let data = b
        .read_file(&file_path.to_string_lossy(), 64 * 1024)
        .unwrap();
    assert_eq!(data.len(), 64 * 1024);
}

#[test]
fn test_backend_metadata_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "Hello, World!").unwrap();

    let b = LocalBackend::new();
    let info = b.metadata(&file_path.to_string_lossy()).unwrap();

    assert!(info.is_file);
    assert!(!info.is_dir);
    assert!(info.size > 0);
    assert!(info.modified.is_some());
}

#[test]
fn test_backend_metadata_directory() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path().join("test_folder");
    fs::create_dir(&dir_path).unwrap();

    let b = LocalBackend::new();
    let info = b.metadata(&dir_path.to_string_lossy()).unwrap();

    assert!(info.is_dir);
    assert!(!info.is_file);
}

#[test]
fn test_backend_metadata_nonexistent() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent = temp_dir.path().join("nonexistent.txt");

    let b = LocalBackend::new();
    assert!(b.metadata(&nonexistent.to_string_lossy()).is_err());
}

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// Import the functions we want to test
// Note: These need to be public in the main crate
use vocofo::file_operation::{create_dir, delete, copy_dir, list_children};
use vocofo::context::Context;

/// Helper function to create a test directory structure
fn setup_test_dir() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create test structure:
    // temp_dir/
    //   ├── file1.txt
    //   ├── file2.txt
    //   ├── folder1/
    //   │   ├── nested_file.txt
    //   │   └── subfolder/
    //   └── folder2/

    let base = temp_dir.path();

    fs::write(base.join("file1.txt"), "content1").unwrap();
    fs::write(base.join("file2.txt"), "content2").unwrap();

    fs::create_dir(base.join("folder1")).unwrap();
    fs::write(base.join("folder1/nested_file.txt"), "nested").unwrap();
    fs::create_dir(base.join("folder1/subfolder")).unwrap();

    fs::create_dir(base.join("folder2")).unwrap();

    temp_dir
}

#[test]
fn test_create_directory() {
    let temp_dir = TempDir::new().unwrap();
    let new_dir = temp_dir.path().join("new_folder");

    // Test creating a directory
    assert!(create_dir(&new_dir).is_ok());
    assert!(new_dir.exists());
    assert!(new_dir.is_dir());
}

#[test]
fn test_create_nested_directory() {
    let temp_dir = TempDir::new().unwrap();
    let nested_dir = temp_dir.path().join("parent/child/grandchild");

    // Should create all parent directories
    assert!(create_dir(&nested_dir).is_ok());
    assert!(nested_dir.exists());
    assert!(nested_dir.is_dir());
}

#[test]
fn test_create_existing_directory() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path().join("existing");

    fs::create_dir(&dir_path).unwrap();

    // Creating an existing directory should succeed (idempotent)
    assert!(create_dir(&dir_path).is_ok());
}

#[test]
fn test_delete_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("delete_me.txt");

    fs::write(&file_path, "content").unwrap();
    assert!(file_path.exists());

    // Delete the file
    assert!(delete(&file_path).is_ok());
    assert!(!file_path.exists());
}

#[test]
fn test_delete_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path().join("empty_dir");

    fs::create_dir(&dir_path).unwrap();
    assert!(dir_path.exists());

    // Delete empty directory
    assert!(delete(&dir_path).is_ok());
    assert!(!dir_path.exists());
}

#[test]
fn test_delete_directory_with_contents() {
    let temp_dir = setup_test_dir();
    let folder_path = temp_dir.path().join("folder1");

    assert!(folder_path.exists());

    // Should delete directory and all contents
    assert!(delete(&folder_path).is_ok());
    assert!(!folder_path.exists());
}

#[test]
fn test_delete_prevents_parent_directory() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, "test").unwrap();

    // Attempting to delete with "../" should fail
    let dangerous_path = temp_dir.path().join("../should_not_delete");
    let result = delete(&dangerous_path);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Cannot delete parent directory"));
}

#[test]
fn test_delete_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent = temp_dir.path().join("does_not_exist.txt");

    // Deleting non-existent file should return an error
    assert!(delete(&nonexistent).is_err());
}

#[test]
fn test_copy_directory_basic() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source");
    let dest = temp_dir.path().join("destination");

    // Create source directory with content
    fs::create_dir(&source).unwrap();
    fs::write(source.join("file.txt"), "content").unwrap();

    // Copy directory
    assert!(copy_dir(&source, &dest).is_ok());

    // Verify destination exists and has content
    assert!(dest.exists());
    assert!(dest.is_dir());
    assert!(dest.join("file.txt").exists());

    let content = fs::read_to_string(dest.join("file.txt")).unwrap();
    assert_eq!(content, "content");
}

#[test]
fn test_copy_directory_recursive() {
    let temp_dir = setup_test_dir();
    let source = temp_dir.path().join("folder1");
    let dest = temp_dir.path().join("folder1_copy");

    // Copy directory with nested structure
    assert!(copy_dir(&source, &dest).is_ok());

    // Verify structure is preserved
    assert!(dest.exists());
    assert!(dest.join("nested_file.txt").exists());
    assert!(dest.join("subfolder").exists());
    assert!(dest.join("subfolder").is_dir());
}

#[test]
fn test_copy_directory_into_itself() {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path().join("folder");
    fs::create_dir(&dir).unwrap();

    let dest = dir.join("folder"); // Same name inside itself

    // Should fail - cannot copy directory into itself
    let result = copy_dir(&dir, &dest);
    assert!(result.is_err());
}

#[test]
fn test_list_children_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let mut context = Context::new().unwrap();
    context.path = temp_dir.path().to_string_lossy().to_string();

    let result = list_children(&mut context);
    assert!(result.is_ok());

    let items = result.unwrap();
    // Should only have "../" entry
    assert_eq!(context.items.len(), 1);
    assert_eq!(context.items[0], "../");
}

#[test]
fn test_list_children_with_files_and_folders() {
    let temp_dir = setup_test_dir();
    let mut context = Context::new().unwrap();
    context.path = temp_dir.path().to_string_lossy().to_string();

    let result = list_children(&mut context);
    assert!(result.is_ok());

    // Should have: ../, folder1/, folder2/, file1.txt, file2.txt
    assert_eq!(context.items.len(), 5);

    // Check that folders come before files and have trailing slash
    assert!(context.items[0] == "../");
    assert!(context.items[1].ends_with('/'));
    assert!(context.items[2].ends_with('/'));

    // Check files don't have trailing slash
    assert!(!context.items[3].ends_with('/'));
    assert!(!context.items[4].ends_with('/'));
}

#[test]
fn test_list_children_sorted() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Create items in non-alphabetical order
    fs::write(base.join("zebra.txt"), "").unwrap();
    fs::write(base.join("apple.txt"), "").unwrap();
    fs::create_dir(base.join("zoo")).unwrap();
    fs::create_dir(base.join("archive")).unwrap();

    let mut context = Context::new().unwrap();
    context.path = base.to_string_lossy().to_string();

    let result = list_children(&mut context);
    assert!(result.is_ok());

    // Items should be sorted: ../, folders (sorted), files (sorted)
    assert_eq!(context.items[0], "../");
    assert_eq!(context.items[1], "archive/");
    assert_eq!(context.items[2], "zoo/");
    assert_eq!(context.items[3], "apple.txt");
    assert_eq!(context.items[4], "zebra.txt");
}

#[test]
fn test_list_children_invalid_path() {
    let mut context = Context::new().unwrap();
    context.path = "/nonexistent/path/that/does/not/exist".to_string();

    let result = list_children(&mut context);
    assert!(result.is_err());
}

#[test]
fn test_list_children_special_characters() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Create files with spaces and special characters
    fs::write(base.join("file with spaces.txt"), "").unwrap();
    fs::write(base.join("file-with-dashes.txt"), "").unwrap();
    fs::create_dir(base.join("folder (copy)")).unwrap();

    let mut context = Context::new().unwrap();
    context.path = base.to_string_lossy().to_string();

    let result = list_children(&mut context);
    assert!(result.is_ok());

    // All items should be listed correctly
    assert!(context.items.iter().any(|i| i == "file with spaces.txt"));
    assert!(context.items.iter().any(|i| i == "file-with-dashes.txt"));
    assert!(context.items.iter().any(|i| i == "folder (copy)/"));
}

use std::fs;
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

    let _items = result.unwrap();
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

// ============================================================================
// Preview Function Tests
// ============================================================================

use vocofo::file_operation::{
    generate_preview, read_file_preview, get_directory_preview,
    format_file_metadata, format_size
};

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

#[test]
fn test_read_file_preview_text_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    let content = "Hello, World!\nThis is a test file.";
    fs::write(&file_path, content).unwrap();

    let result = read_file_preview(&file_path);
    assert!(result.is_ok());

    let preview = result.unwrap();
    assert_eq!(preview, content);
}

#[test]
fn test_read_file_preview_large_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("large.txt");

    // Create a file larger than 64KB
    let large_content = "A".repeat(70 * 1024); // 70KB
    fs::write(&file_path, &large_content).unwrap();

    let result = read_file_preview(&file_path);
    assert!(result.is_ok());

    let preview = result.unwrap();
    // Should only read first 64KB
    assert_eq!(preview.len(), 64 * 1024);
}

#[test]
fn test_read_file_preview_binary_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("binary.bin");

    // Create a binary file with non-UTF8 bytes
    let binary_data: Vec<u8> = vec![0xFF, 0xFE, 0xFD, 0xFC, 0x00, 0x01, 0x02];
    fs::write(&file_path, binary_data).unwrap();

    let result = read_file_preview(&file_path);
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(error.to_string().contains("Binary file"));
}

#[test]
fn test_read_file_preview_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.txt");

    fs::write(&file_path, "").unwrap();

    let result = read_file_preview(&file_path);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "");
}

#[test]
fn test_read_file_preview_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("nonexistent.txt");

    let result = read_file_preview(&file_path);
    assert!(result.is_err());
}

#[test]
fn test_get_directory_preview_empty_directory() {
    let temp_dir = TempDir::new().unwrap();

    let result = get_directory_preview(temp_dir.path());
    assert!(result.is_ok());

    let items = result.unwrap();
    assert_eq!(items.len(), 0);
}

#[test]
fn test_get_directory_preview_few_items() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Create a few files and folders
    fs::write(base.join("file1.txt"), "").unwrap();
    fs::write(base.join("file2.txt"), "").unwrap();
    fs::create_dir(base.join("folder1")).unwrap();

    let result = get_directory_preview(base);
    assert!(result.is_ok());

    let items = result.unwrap();
    assert_eq!(items.len(), 3);

    // Folders should come first
    assert_eq!(items[0], "folder1/");
    assert!(items.contains(&"file1.txt".to_string()));
    assert!(items.contains(&"file2.txt".to_string()));
}

#[test]
fn test_get_directory_preview_truncation() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Create more than 20 items (the limit)
    for i in 0..25 {
        fs::write(base.join(format!("file{}.txt", i)), "").unwrap();
    }

    let result = get_directory_preview(base);
    assert!(result.is_ok());

    let items = result.unwrap();
    // Should have 20 items + 1 "... and N more" message
    assert_eq!(items.len(), 21);

    let last_item = items.last().unwrap();
    assert!(last_item.contains("... and"));
    assert!(last_item.contains("5 more items"));
}

#[test]
fn test_get_directory_preview_sorting() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Create items in non-alphabetical order
    fs::write(base.join("zebra.txt"), "").unwrap();
    fs::create_dir(base.join("delta")).unwrap();
    fs::write(base.join("alpha.txt"), "").unwrap();
    fs::create_dir(base.join("bravo")).unwrap();

    let result = get_directory_preview(base);
    assert!(result.is_ok());

    let items = result.unwrap();
    // Folders first (alphabetically), then files (alphabetically)
    assert_eq!(items[0], "bravo/");
    assert_eq!(items[1], "delta/");
    assert_eq!(items[2], "alpha.txt");
    assert_eq!(items[3], "zebra.txt");
}

#[test]
fn test_format_file_metadata_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    fs::write(&file_path, "Hello, World!").unwrap();

    let metadata_str = format_file_metadata(&file_path);

    assert!(metadata_str.contains("Type: File"));
    assert!(metadata_str.contains("Size:"));
    assert!(metadata_str.contains("Modified:"));
    assert!(metadata_str.contains("Permissions:"));
}

#[test]
fn test_format_file_metadata_directory() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path().join("test_folder");

    fs::create_dir(&dir_path).unwrap();

    let metadata_str = format_file_metadata(&dir_path);

    assert!(metadata_str.contains("Type: Directory"));
    assert!(metadata_str.contains("Size:"));
    assert!(metadata_str.contains("Modified:"));
}

#[test]
fn test_format_file_metadata_nonexistent() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent = temp_dir.path().join("nonexistent.txt");

    let metadata_str = format_file_metadata(&nonexistent);

    assert!(metadata_str.contains("Error reading metadata"));
}

#[test]
fn test_generate_preview_text_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    fs::write(&file_path, "Hello, World!").unwrap();

    let preview = generate_preview(&file_path);

    assert!(preview.contains("Type: File"));
    assert!(preview.contains("Preview"));
    assert!(preview.contains("Hello, World!"));
}

#[test]
fn test_generate_preview_binary_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("binary.bin");

    let binary_data: Vec<u8> = vec![0xFF, 0xFE, 0xFD];
    fs::write(&file_path, binary_data).unwrap();

    let preview = generate_preview(&file_path);

    assert!(preview.contains("Type: File"));
    assert!(preview.contains("Binary file"));
}

#[test]
fn test_generate_preview_directory() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Create some files in the directory
    fs::write(base.join("file1.txt"), "").unwrap();
    fs::write(base.join("file2.txt"), "").unwrap();

    let preview = generate_preview(base);

    assert!(preview.contains("Type: Directory"));
    assert!(preview.contains("Contents"));
    assert!(preview.contains("file1.txt"));
    assert!(preview.contains("file2.txt"));
}

#[test]
fn test_generate_preview_empty_directory() {
    let temp_dir = TempDir::new().unwrap();

    let preview = generate_preview(temp_dir.path());

    assert!(preview.contains("Type: Directory"));
    assert!(preview.contains("Contents"));
}

#[test]
fn test_generate_preview_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent = temp_dir.path().join("nonexistent.txt");

    let preview = generate_preview(&nonexistent);

    assert!(preview.contains("File not found"));
}

#[test]
fn test_directory_size_calculation() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Create files and a subdirectory
    fs::write(base.join("file1.txt"), "a".repeat(1000)).unwrap();
    fs::write(base.join("file2.txt"), "b".repeat(2000)).unwrap();
    fs::create_dir(base.join("subdir")).unwrap();

    let metadata_str = format_file_metadata(base);

    assert!(metadata_str.contains("Type: Directory"));
    // Directories now show item count instead of recursive size
    assert!(metadata_str.contains("3 items"));
}

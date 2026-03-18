use std::fs;
use tempfile::TempDir;
use vocofo::context::Context;
use vocofo::file_operation;

/// Integration test for complete copy/paste workflow
#[test]
fn test_copy_paste_file_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Setup: Create source and destination directories
    let source_dir = base.join("source");
    let dest_dir = base.join("dest");
    fs::create_dir(&source_dir).unwrap();
    fs::create_dir(&dest_dir).unwrap();

    // Create a test file
    let test_file = source_dir.join("test.txt");
    fs::write(&test_file, "test content").unwrap();

    // Step 1: Navigate to source directory
    let mut context = Context::new().unwrap();
    context.panels[0].path = source_dir.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    // Step 2: Select the file (should be after "../")
    let file_idx = context.panels[0].items.iter().position(|i| i == "test.txt").unwrap();
    context.panels[0].state = file_idx;

    // Step 3: Copy (Ctrl+C simulation)
    context.set_copy_path();
    assert!(!context.copy_path.is_empty());
    assert!(context.copy_path.contains("test.txt"));

    // Step 4: Navigate to destination directory
    context.panels[0].path = dest_dir.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    // Step 5: Select destination (../ for current directory)
    context.panels[0].state = 0;

    // Step 6: Paste (Ctrl+V simulation)
    let result = file_operation::copy_file(&mut context);
    assert!(result.is_ok(), "Copy operation failed: {:?}", result.err());

    // Verify: File exists in destination
    let copied_file = dest_dir.join("test.txt");
    assert!(copied_file.exists());

    let content = fs::read_to_string(&copied_file).unwrap();
    assert_eq!(content, "test content");
}

#[test]
fn test_copy_paste_folder_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Setup: Create source directory with nested content
    let source_dir = base.join("source");
    fs::create_dir(&source_dir).unwrap();

    let test_folder = source_dir.join("myfolder");
    fs::create_dir(&test_folder).unwrap();
    fs::write(test_folder.join("file1.txt"), "content1").unwrap();
    fs::write(test_folder.join("file2.txt"), "content2").unwrap();

    let dest_dir = base.join("dest");
    fs::create_dir(&dest_dir).unwrap();

    // Step 1: Navigate to source and select folder
    let mut context = Context::new().unwrap();
    context.panels[0].path = source_dir.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    let folder_idx = context.panels[0].items.iter().position(|i| i == "myfolder/").unwrap();
    context.panels[0].state = folder_idx;

    // Step 2: Copy folder
    context.set_copy_path();

    // Step 3: Navigate to destination and paste
    context.panels[0].path = dest_dir.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();
    context.panels[0].state = 0;

    let result = file_operation::copy_file(&mut context);
    assert!(result.is_ok(), "Copy folder failed: {:?}", result.err());

    // Verify: Folder and contents exist in destination
    let copied_folder = dest_dir.join("myfolder");
    assert!(copied_folder.exists());
    assert!(copied_folder.is_dir());
    assert!(copied_folder.join("file1.txt").exists());
    assert!(copied_folder.join("file2.txt").exists());

    let content1 = fs::read_to_string(copied_folder.join("file1.txt")).unwrap();
    assert_eq!(content1, "content1");
}

#[test]
fn test_copy_paste_into_subfolder() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Setup
    fs::write(base.join("file.txt"), "content").unwrap();
    fs::create_dir(base.join("subfolder")).unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = base.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    // Copy file
    let file_idx = context.panels[0].items.iter().position(|i| i == "file.txt").unwrap();
    context.panels[0].state = file_idx;
    context.set_copy_path();

    // Navigate to same directory, select subfolder
    let folder_idx = context.panels[0].items.iter().position(|i| i == "subfolder/").unwrap();
    context.panels[0].state = folder_idx;

    // Paste into subfolder
    let result = file_operation::copy_file(&mut context);
    assert!(result.is_ok());

    // Verify
    let copied = base.join("subfolder/file.txt");
    assert!(copied.exists());
}

#[test]
fn test_copy_paste_same_directory_fails() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    fs::write(base.join("file.txt"), "content").unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = base.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    // Copy file
    let file_idx = context.panels[0].items.iter().position(|i| i == "file.txt").unwrap();
    context.panels[0].state = file_idx;
    context.set_copy_path();

    // Try to paste in same location
    context.panels[0].state = 0; // Select "../" (current directory)
    let result = file_operation::copy_file(&mut context);

    // Should fail with "Destination already exists"
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}

#[test]
fn test_copy_without_selection() {
    let mut context = Context::new().unwrap();

    // Try to copy with no items in list
    context.set_copy_path();

    // Should not crash, clipboard should remain empty
    assert!(context.copy_path.is_empty());
}

#[test]
fn test_paste_with_empty_clipboard() {
    let temp_dir = TempDir::new().unwrap();
    let mut context = Context::new().unwrap();
    context.panels[0].path = temp_dir.path().to_string_lossy().to_string();

    // Clipboard is empty
    assert!(context.copy_path.is_empty());

    // Try to paste - should fail gracefully
    let result = file_operation::copy_file(&mut context);
    assert!(result.is_err());
}

#[test]
fn test_copy_parent_directory_does_nothing() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    let mut context = Context::new().unwrap();
    context.panels[0].path = base.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    // Select "../"
    context.panels[0].state = 0;
    assert_eq!(context.panels[0].get_selected_item(), Some(&"../".to_string()));

    // Try to copy
    context.set_copy_path();

    // Clipboard should remain empty
    assert!(context.copy_path.is_empty());
}

#[test]
fn test_copy_file_with_special_characters() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Create file with spaces and special chars
    let special_file = "file with spaces (copy).txt";
    fs::write(base.join(special_file), "content").unwrap();

    let dest = base.join("dest");
    fs::create_dir(&dest).unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = base.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    // Find and copy the special file
    let file_idx = context.panels[0].items.iter().position(|i| i == special_file).unwrap();
    context.panels[0].state = file_idx;
    context.set_copy_path();

    // Paste to dest
    context.panels[0].path = dest.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();
    context.panels[0].state = 0;

    let result = file_operation::copy_file(&mut context);
    assert!(result.is_ok());

    // Verify
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

    // Copy file
    let file_idx = context.panels[0].items.iter().position(|i| i == "file.txt").unwrap();
    context.panels[0].state = file_idx;
    context.set_copy_path();

    let clipboard_content = context.copy_path.clone();
    assert!(!clipboard_content.is_empty());

    // Navigate to folder1
    context.panels[0].path = base.join("folder1").to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    // Clipboard should still have the file
    assert_eq!(context.copy_path, clipboard_content);

    // Navigate to folder2
    context.panels[0].path = base.join("folder2").to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    // Clipboard should still persist
    assert_eq!(context.copy_path, clipboard_content);
}

#[test]
fn test_overwrite_existing_file_fails() {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();

    // Create same file in source and dest
    fs::write(base.join("file.txt"), "original").unwrap();

    let dest = base.join("dest");
    fs::create_dir(&dest).unwrap();
    fs::write(dest.join("file.txt"), "existing").unwrap();

    let mut context = Context::new().unwrap();
    context.panels[0].path = base.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();

    // Copy file
    let file_idx = context.panels[0].items.iter().position(|i| i == "file.txt").unwrap();
    context.panels[0].state = file_idx;
    context.set_copy_path();

    // Try to paste to dest (where file already exists)
    context.panels[0].path = dest.to_string_lossy().to_string();
    file_operation::list_children(&mut context.panels[0]).unwrap();
    context.panels[0].state = 0;

    let result = file_operation::copy_file(&mut context);

    // Should fail - destination already exists
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));

    // Original file in dest should be unchanged
    let content = fs::read_to_string(dest.join("file.txt")).unwrap();
    assert_eq!(content, "existing");
}

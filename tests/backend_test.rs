use std::fs;
use tempfile::TempDir;
use vocofo::backend::FilesystemBackend;
use vocofo::local_backend::LocalBackend;

fn setup() -> (LocalBackend, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let base = temp_dir.path();
    fs::write(base.join("file1.txt"), "hello world").unwrap();
    fs::write(base.join("file2.txt"), "content").unwrap();
    fs::create_dir(base.join("subdir")).unwrap();
    fs::write(base.join("subdir").join("nested.txt"), "nested").unwrap();
    (LocalBackend::new(), temp_dir)
}

#[test]
fn test_display_name() {
    let backend = LocalBackend::new();
    assert_eq!(backend.display_name(), "Local");
}

#[test]
fn test_is_local() {
    let backend = LocalBackend::new();
    assert!(backend.is_local());
}

#[test]
fn test_list_dir() {
    let (backend, temp_dir) = setup();
    let path = temp_dir.path().to_string_lossy().to_string();
    let entries = backend.list_dir(&path).unwrap();
    assert_eq!(entries.len(), 3); // file1.txt, file2.txt, subdir
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"file1.txt"));
    assert!(names.contains(&"file2.txt"));
    assert!(names.contains(&"subdir"));
}

#[test]
fn test_list_dir_metadata() {
    let (backend, temp_dir) = setup();
    let path = temp_dir.path().to_string_lossy().to_string();
    let entries = backend.list_dir(&path).unwrap();
    let subdir = entries.iter().find(|e| e.name == "subdir").unwrap();
    assert!(subdir.info.is_dir);
    assert!(!subdir.info.is_file);

    let file = entries.iter().find(|e| e.name == "file1.txt").unwrap();
    assert!(!file.info.is_dir);
    assert!(file.info.is_file);
    assert_eq!(file.info.size, 11); // "hello world"
}

#[test]
fn test_metadata() {
    let (backend, temp_dir) = setup();
    let path = temp_dir
        .path()
        .join("file1.txt")
        .to_string_lossy()
        .to_string();
    let info = backend.metadata(&path).unwrap();
    assert!(info.is_file);
    assert!(!info.is_dir);
    assert_eq!(info.size, 11);
}

#[test]
fn test_metadata_dir() {
    let (backend, temp_dir) = setup();
    let path = temp_dir.path().join("subdir").to_string_lossy().to_string();
    let info = backend.metadata(&path).unwrap();
    assert!(info.is_dir);
    assert!(!info.is_file);
}

#[test]
fn test_exists() {
    let (backend, temp_dir) = setup();
    let base = temp_dir.path().to_string_lossy().to_string();
    assert!(backend.exists(&format!("{}/file1.txt", base)).unwrap());
    assert!(!backend.exists(&format!("{}/nonexistent", base)).unwrap());
}

#[test]
fn test_canonicalize() {
    let (backend, temp_dir) = setup();
    let path = temp_dir
        .path()
        .join("subdir")
        .join("..")
        .to_string_lossy()
        .to_string();
    let canonical = backend.canonicalize(&path).unwrap();
    let expected = temp_dir
        .path()
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .to_string();
    assert_eq!(canonical, expected);
}

#[test]
fn test_read_file() {
    let (backend, temp_dir) = setup();
    let path = temp_dir
        .path()
        .join("file1.txt")
        .to_string_lossy()
        .to_string();
    let data = backend.read_file(&path, 1024).unwrap();
    assert_eq!(data, b"hello world");
}

#[test]
fn test_read_file_limited() {
    let (backend, temp_dir) = setup();
    let path = temp_dir
        .path()
        .join("file1.txt")
        .to_string_lossy()
        .to_string();
    let data = backend.read_file(&path, 5).unwrap();
    assert_eq!(data, b"hello");
}

#[test]
fn test_write_file() {
    let (backend, temp_dir) = setup();
    let path = temp_dir
        .path()
        .join("new.txt")
        .to_string_lossy()
        .to_string();
    backend.write_file(&path, b"new content").unwrap();
    assert_eq!(
        fs::read_to_string(temp_dir.path().join("new.txt")).unwrap(),
        "new content"
    );
}

#[test]
fn test_create_dir() {
    let (backend, temp_dir) = setup();
    let path = temp_dir
        .path()
        .join("newdir")
        .join("nested")
        .to_string_lossy()
        .to_string();
    backend.create_dir(&path).unwrap();
    assert!(temp_dir.path().join("newdir").join("nested").is_dir());
}

#[test]
fn test_create_file() {
    let (backend, temp_dir) = setup();
    let path = temp_dir
        .path()
        .join("created.txt")
        .to_string_lossy()
        .to_string();
    backend.create_file(&path).unwrap();
    assert!(temp_dir.path().join("created.txt").exists());
    assert_eq!(
        fs::read_to_string(temp_dir.path().join("created.txt")).unwrap(),
        ""
    );
}

#[test]
fn test_remove_file() {
    let (backend, temp_dir) = setup();
    let path = temp_dir
        .path()
        .join("file1.txt")
        .to_string_lossy()
        .to_string();
    backend.remove_file(&path).unwrap();
    assert!(!temp_dir.path().join("file1.txt").exists());
}

#[test]
fn test_remove_dir_all() {
    let (backend, temp_dir) = setup();
    let path = temp_dir.path().join("subdir").to_string_lossy().to_string();
    backend.remove_dir_all(&path).unwrap();
    assert!(!temp_dir.path().join("subdir").exists());
}

#[test]
fn test_rename() {
    let (backend, temp_dir) = setup();
    let from = temp_dir
        .path()
        .join("file1.txt")
        .to_string_lossy()
        .to_string();
    let to = temp_dir
        .path()
        .join("renamed.txt")
        .to_string_lossy()
        .to_string();
    backend.rename(&from, &to).unwrap();
    assert!(!temp_dir.path().join("file1.txt").exists());
    assert!(temp_dir.path().join("renamed.txt").exists());
}

#[test]
fn test_copy_file() {
    let (backend, temp_dir) = setup();
    let from = temp_dir
        .path()
        .join("file1.txt")
        .to_string_lossy()
        .to_string();
    let to = temp_dir
        .path()
        .join("copy.txt")
        .to_string_lossy()
        .to_string();
    backend.copy_file(&from, &to).unwrap();
    assert_eq!(
        fs::read_to_string(temp_dir.path().join("copy.txt")).unwrap(),
        "hello world"
    );
}

#[test]
fn test_copy_dir() {
    let (backend, temp_dir) = setup();
    let from = temp_dir.path().join("subdir").to_string_lossy().to_string();
    let to = temp_dir
        .path()
        .join("subdir_copy")
        .to_string_lossy()
        .to_string();
    backend.copy_dir(&from, &to).unwrap();
    assert!(temp_dir.path().join("subdir_copy").is_dir());
    assert_eq!(
        fs::read_to_string(temp_dir.path().join("subdir_copy").join("nested.txt")).unwrap(),
        "nested"
    );
}

#[test]
fn test_join_path() {
    let backend = LocalBackend::new();
    let result = backend.join_path("/home/user", "file.txt");
    assert_eq!(result, "/home/user/file.txt");
}

#[test]
fn test_parent_path() {
    let backend = LocalBackend::new();
    assert_eq!(
        backend.parent_path("/home/user/file.txt"),
        Some("/home/user".to_string())
    );
    assert_eq!(backend.parent_path("/"), None);
}

#[test]
fn test_file_name() {
    let backend = LocalBackend::new();
    assert_eq!(
        backend.file_name("/home/user/file.txt"),
        Some("file.txt".to_string())
    );
    assert_eq!(
        backend.file_name("/home/user/dir/"),
        Some("dir".to_string())
    );
}

#[test]
fn test_list_dir_nonexistent() {
    let backend = LocalBackend::new();
    assert!(backend.list_dir("/nonexistent/path/xyz").is_err());
}

#[test]
fn test_metadata_nonexistent() {
    let backend = LocalBackend::new();
    assert!(backend.metadata("/nonexistent/path/xyz").is_err());
}

#[test]
fn test_read_file_nonexistent() {
    let backend = LocalBackend::new();
    assert!(backend.read_file("/nonexistent/path/xyz", 1024).is_err());
}

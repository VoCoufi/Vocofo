use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::{fs, io};

use crate::backend::FilesystemBackend;
use crate::file_operation;

/// Shared progress tracking for file transfers
pub struct TransferProgress {
    pub bytes_transferred: AtomicU64,
    pub total_bytes: AtomicU64,
}

impl TransferProgress {
    pub fn new() -> Self {
        Self {
            bytes_transferred: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
        }
    }
}

pub struct FileOpResult {
    pub description: String,
    pub result: Result<(), String>,
    pub clear_clipboard: bool,
}

/// Spawn a copy operation in a background thread
pub fn spawn_copy(from: PathBuf, to: PathBuf, description: String) -> mpsc::Receiver<FileOpResult> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = copy_standalone(&from, &to);
        let _ = tx.send(FileOpResult {
            description,
            result: result.map_err(|e| e.to_string()),
            clear_clipboard: false,
        });
    });
    rx
}

/// Spawn a copy operation using backends (supports cross-backend transfers)
pub fn spawn_copy_with_backend(
    src_backend: Arc<dyn FilesystemBackend>,
    dst_backend: Arc<dyn FilesystemBackend>,
    from: String,
    to: String,
    description: String,
    progress: Option<Arc<TransferProgress>>,
) -> mpsc::Receiver<FileOpResult> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        // Set total size for progress tracking
        if let Some(ref p) = progress {
            if let Ok(info) = src_backend.metadata(&from) {
                p.total_bytes.store(info.size, Ordering::Relaxed);
            }
        }
        let result = if Arc::ptr_eq(&src_backend, &dst_backend) {
            // Same backend: use native copy
            let info = src_backend.metadata(&from)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>);
            match info {
                Ok(info) if info.is_dir => src_backend.copy_dir(&from, &to)
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>),
                Ok(_) => src_backend.copy_file(&from, &to)
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>),
                Err(e) => Err(e),
            }
        } else {
            // Cross-backend: read from source, write to destination
            cross_backend_copy_with_progress(&src_backend, &dst_backend, &from, &to, progress.as_ref())
        };
        let _ = tx.send(FileOpResult {
            description,
            result: result.map_err(|e| e.to_string()),
            clear_clipboard: false,
        });
    });
    rx
}

/// Spawn a move operation using backends
pub fn spawn_move_with_backend(
    src_backend: Arc<dyn FilesystemBackend>,
    dst_backend: Arc<dyn FilesystemBackend>,
    from: String,
    to: String,
    description: String,
    progress: Option<Arc<TransferProgress>>,
) -> mpsc::Receiver<FileOpResult> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = move_with_backend_inner(&src_backend, &dst_backend, &from, &to, progress.as_ref());
        let _ = tx.send(FileOpResult {
            description,
            result: result.map_err(|e| e.to_string()),
            clear_clipboard: true,
        });
    });
    rx
}

/// Spawn a delete operation using a backend
pub fn spawn_delete_with_backend(
    backend: Arc<dyn FilesystemBackend>,
    path: String,
    description: String,
) -> mpsc::Receiver<FileOpResult> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = file_operation::delete_with_backend(&backend, &path);
        let _ = tx.send(FileOpResult {
            description,
            result: result.map_err(|e| e.to_string()),
            clear_clipboard: false,
        });
    });
    rx
}

/// Spawn a batch delete operation using a backend
pub fn spawn_delete_batch_with_backend(
    backend: Arc<dyn FilesystemBackend>,
    paths: Vec<String>,
    description: String,
) -> mpsc::Receiver<FileOpResult> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let total = paths.len();
        let mut errors = Vec::new();
        for path in &paths {
            if let Err(e) = file_operation::delete_with_backend(&backend, path) {
                errors.push(format!("{}: {}", path, e));
            }
        }
        let result = if errors.is_empty() {
            Ok(())
        } else {
            let failed = errors.len();
            Err(format!("{} of {} failed: {}", failed, total, errors.join("; ")))
        };
        let _ = tx.send(FileOpResult {
            description,
            result,
            clear_clipboard: false,
        });
    });
    rx
}

/// Spawn a batch copy/move operation using backends
pub fn spawn_copy_batch_with_backend(
    src_backend: Arc<dyn FilesystemBackend>,
    dst_backend: Arc<dyn FilesystemBackend>,
    items: Vec<(String, String)>,
    description: String,
    is_move: bool,
    progress: Option<Arc<TransferProgress>>,
) -> mpsc::Receiver<FileOpResult> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        // Calculate total size for progress
        if let Some(ref p) = progress {
            let mut total: u64 = 0;
            for (from, _) in &items {
                if let Ok(info) = src_backend.metadata(from) {
                    total += info.size;
                }
            }
            p.total_bytes.store(total, Ordering::Relaxed);
        }
        let mut errors = Vec::new();
        for (from, to) in &items {
            let copy_result = if Arc::ptr_eq(&src_backend, &dst_backend) {
                let info = match src_backend.metadata(from) {
                    Ok(i) => i,
                    Err(e) => { errors.push(format!("{}: {}", from, e)); continue; }
                };
                if info.is_dir {
                    src_backend.copy_dir(from, to)
                } else {
                    src_backend.copy_file(from, to)
                }.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
            } else {
                cross_backend_copy_with_progress(&src_backend, &dst_backend, from, to, progress.as_ref())
            };

            if let Err(e) = copy_result {
                errors.push(format!("{}: {}", from, e));
                continue;
            }
            if is_move {
                if let Err(e) = file_operation::delete_with_backend(&src_backend, from) {
                    errors.push(format!("delete {}: {}", from, e));
                }
            }
        }
        let result = if errors.is_empty() {
            Ok(())
        } else {
            let failed = errors.len();
            let total = items.len();
            Err(format!("{} of {} failed: {}", failed, total, errors.join("; ")))
        };
        let _ = tx.send(FileOpResult {
            description,
            result,
            clear_clipboard: is_move,
        });
    });
    rx
}

fn move_with_backend_inner(
    src_backend: &Arc<dyn FilesystemBackend>,
    dst_backend: &Arc<dyn FilesystemBackend>,
    from: &str,
    to: &str,
    progress: Option<&Arc<TransferProgress>>,
) -> Result<(), Box<dyn std::error::Error>> {
    if Arc::ptr_eq(src_backend, dst_backend) {
        // Same backend: try rename first, fallback to copy+delete
        if src_backend.rename(from, to).is_ok() {
            return Ok(());
        }
        let info = src_backend.metadata(from)?;
        if info.is_dir {
            src_backend.copy_dir(from, to)?;
            src_backend.remove_dir_all(from)?;
        } else {
            src_backend.copy_file(from, to)?;
            src_backend.remove_file(from)?;
        }
    } else {
        // Cross-backend: copy then delete source
        cross_backend_copy_with_progress(src_backend, dst_backend, from, to, progress)?;
        file_operation::delete_with_backend(src_backend, from)?;
    }
    Ok(())
}

/// Cross-backend copy: read from source backend, write to destination backend
fn cross_backend_copy(
    src: &Arc<dyn FilesystemBackend>,
    dst: &Arc<dyn FilesystemBackend>,
    from: &str,
    to: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    cross_backend_copy_with_progress(src, dst, from, to, None)
}

/// Cross-backend copy with optional progress tracking
fn cross_backend_copy_with_progress(
    src: &Arc<dyn FilesystemBackend>,
    dst: &Arc<dyn FilesystemBackend>,
    from: &str,
    to: &str,
    progress: Option<&Arc<TransferProgress>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let info = src.metadata(from)?;

    if info.is_dir {
        dst.create_dir(to)?;
        let entries = src.list_dir(from)?;
        for entry in entries {
            let src_child = src.join_path(from, &entry.name);
            let dst_child = dst.join_path(to, &entry.name);
            cross_backend_copy_with_progress(src, dst, &src_child, &dst_child, progress)?;
        }
    } else {
        let data = src.read_file(from, info.size as usize)?;
        let len = data.len() as u64;
        dst.write_file(to, &data)?;
        if let Some(p) = progress {
            p.bytes_transferred.fetch_add(len, Ordering::Relaxed);
        }
    }

    Ok(())
}

// === Legacy functions kept for backward compatibility with existing tests ===

/// Spawn a move (copy + delete) operation in a background thread
pub fn spawn_move(from: PathBuf, to: PathBuf, description: String) -> mpsc::Receiver<FileOpResult> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = copy_standalone(&from, &to).and_then(|_| {
            file_operation::delete(&from)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        });
        let _ = tx.send(FileOpResult {
            description,
            result: result.map_err(|e| e.to_string()),
            clear_clipboard: true,
        });
    });
    rx
}

/// Spawn a delete operation in a background thread
pub fn spawn_delete(path: PathBuf, description: String) -> mpsc::Receiver<FileOpResult> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = file_operation::delete(&path)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>);
        let _ = tx.send(FileOpResult {
            description,
            result: result.map_err(|e| e.to_string()),
            clear_clipboard: false,
        });
    });
    rx
}

/// Spawn a batch delete operation in a background thread
pub fn spawn_delete_batch(paths: Vec<PathBuf>, description: String) -> mpsc::Receiver<FileOpResult> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let total = paths.len();
        let mut errors = Vec::new();
        for path in &paths {
            if let Err(e) = file_operation::delete(path) {
                errors.push(format!("{}: {}", path.display(), e));
            }
        }
        let result = if errors.is_empty() {
            Ok(())
        } else {
            let failed = errors.len();
            Err(format!("{} of {} failed: {}", failed, total, errors.join("; ")))
        };
        let _ = tx.send(FileOpResult {
            description,
            result,
            clear_clipboard: false,
        });
    });
    rx
}

/// Spawn a batch copy operation in a background thread
pub fn spawn_copy_batch(items: Vec<(PathBuf, PathBuf)>, description: String, is_move: bool) -> mpsc::Receiver<FileOpResult> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let total = items.len();
        let mut errors = Vec::new();
        for (from, to) in &items {
            if let Err(e) = copy_standalone(from, to) {
                errors.push(format!("{}: {}", from.display(), e));
                continue;
            }
            if is_move {
                if let Err(e) = file_operation::delete(from) {
                    errors.push(format!("delete {}: {}", from.display(), e));
                }
            }
        }
        let result = if errors.is_empty() {
            Ok(())
        } else {
            let failed = errors.len();
            Err(format!("{} of {} failed: {}", failed, total, errors.join("; ")))
        };
        let _ = tx.send(FileOpResult {
            description,
            result,
            clear_clipboard: is_move,
        });
    });
    rx
}

/// Standalone copy that works on paths only (no Context needed)
fn copy_standalone(from: &PathBuf, to: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if from == to {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Source and destination are the same",
        )));
    }

    let meta = fs::metadata(from)?;
    if meta.is_dir() {
        file_operation::copy_dir(from, to)?;
    } else {
        if to.exists() {
            return Err(Box::new(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Destination already exists",
            )));
        }
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(from, to)?;
    }
    Ok(())
}

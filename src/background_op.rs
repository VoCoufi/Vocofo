use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::{fs, io};

use crate::file_operation;

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

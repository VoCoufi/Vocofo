use std::io::Result;
use std::path::{Path, PathBuf};
use std::{fs, io};
use std::sync::Arc;

use crate::backend::{FileInfo, FilesystemBackend};
use crate::context::UiState::Normal;
use crate::context::{Context, PanelState};

/// Result type for file operations that can return any error type
pub type FileResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Reads the directory via the panel's backend and populates `panel.items`.
/// Folders (blue) are listed before files (green), both sorted alphabetically.
///
/// ### Examples:
///
/// ```rust,no_run
/// use vocofo::context::Context;
/// use vocofo::file_operation::list_children;
///
/// let mut context = Context::new().unwrap();
/// list_children(context.active_mut()).unwrap();
/// println!("Found {} items", context.active().items.len());
/// ```
pub fn list_children(panel: &mut PanelState) -> Result<()> {
    let mut folders = vec!["../".to_string()];
    let mut files = Vec::new();

    panel.items.clear();

    let entries = panel.backend.list_dir(&panel.path)?;
    for entry in entries {
        // Filter hidden files unless show_hidden is enabled
        if !panel.show_hidden && entry.name.starts_with('.') {
            continue;
        }

        if entry.info.is_dir {
            folders.push(format!("{}/", entry.name));
        } else {
            files.push(entry.name);
        }
    }

    folders.sort();
    files.sort();

    panel.items.extend(folders);
    panel.items.extend(files);
    panel.items_dirty = false;
    panel.apply_filter();

    Ok(())
}

/// Deletes using a backend
pub fn delete_with_backend(backend: &Arc<dyn FilesystemBackend>, path: &str) -> Result<()> {
    if path.contains("../") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Cannot delete parent directory"
        ));
    }

    let info = backend.metadata(path)?;
    if info.is_dir {
        backend.remove_dir_all(path)
    } else {
        backend.remove_file(path)
    }
}

/// Returns a short size string using backend metadata
pub fn format_item_details_from_info(info: &FileInfo) -> String {
    if info.is_dir {
        String::new() // item count requires listing, skip for now
    } else {
        format_size(info.size)
    }
}

/// RAII guard that removes a temp file on drop (even on panic/early return)
struct TempFileGuard(PathBuf);

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Opens a file using the appropriate backend
pub fn open_file_with_backend(backend: &Arc<dyn FilesystemBackend>, path: &str) -> FileResult<()> {
    if backend.is_local() {
        edit::edit_file(Path::new(path))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    } else {
        // Remote: download to temp, edit, upload back
        let data = backend.read_file(path, usize::MAX)?;
        let tmp_dir = std::env::temp_dir();
        let file_name = backend.file_name(path).unwrap_or_else(|| "tempfile".to_string());
        let tmp_path = tmp_dir.join(&file_name);
        fs::write(&tmp_path, &data)?;
        let _guard = TempFileGuard(tmp_path.clone());
        edit::edit_file(&tmp_path)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        let modified = fs::read(&tmp_path)?;
        backend.write_file(path, &modified)?;
        Ok(())
    }
}



/// Handle creating a new directory from the user input
pub fn handle_create_directory(context: &mut Context) -> FileResult<()> {
    let input = context.get_input()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("No input provided"))?;

    let path = context.active().backend.join_path(&context.active().path, input);

    context.set_ui_state(Normal);
    context.active().backend.create_dir(&path)?;
    context.set_input(String::default());
    context.active_mut().state = 0;

    Ok(())
}

/// Handle creating a new file from the user input
pub fn handle_create_file(context: &mut Context) -> FileResult<()> {
    let input = context.get_input()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("No input provided"))?;

    if input.is_empty() {
        return Err("Filename cannot be empty".into());
    }

    let path = context.active().backend.join_path(&context.active().path, input);

    if context.active().backend.exists(&path).unwrap_or(false) {
        return Err(format!("Already exists: {}", path).into());
    }

    context.set_ui_state(Normal);
    context.active().backend.create_file(&path)?;
    context.set_input(String::default());
    context.active_mut().state = 0;

    Ok(())
}

/// Handle rename operation from user input
pub fn handle_rename(context: &mut Context) -> FileResult<()> {
    let new_name = context.get_input()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("No input provided"))?
        .clone();

    if new_name.is_empty() {
        return Err("Name cannot be empty".into());
    }

    let panel = context.active();
    let selected = panel.get_selected_item()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("No item selected"))?
        .clone();

    let old_path = panel.backend.join_path(&panel.path, selected.trim_end_matches('/'));
    let new_path = panel.backend.join_path(&panel.path, &new_name);

    context.set_ui_state(Normal);
    context.active().backend.rename(&old_path, &new_path)?;
    context.set_input(String::default());

    Ok(())
}

/// Resolve source and destination paths for a paste operation
pub fn resolve_paste_paths(context: &mut Context) -> FileResult<(String, String)> {
    let from = context.get_copy_path().clone();
    let dest_dir = resolve_dest_dir(context)?;
    let src_backend = context.copy_source_backend.as_ref()
        .unwrap_or(&context.active().backend);
    let item_name = src_backend.file_name(&from)
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Invalid source path"))?;
    let to = context.active().backend.join_path(&dest_dir, &item_name);
    Ok((from, to))
}

fn resolve_dest_dir(context: &mut Context) -> FileResult<String> {
    let panel = context.active();
    let base_path = panel.path.clone();

    if panel.get_state() != 0 {
        if let Some(item) = panel.get_selected_item() {
            let full = panel.backend.join_path(&base_path, item.trim_end_matches('/'));
            if let Ok(info) = panel.backend.metadata(&full) {
                if info.is_dir {
                    return Ok(full);
                }
            }
        }
    }

    Ok(base_path)
}

/// Formats metadata from a FileInfo (backend-agnostic)
pub fn format_file_metadata_from_info(info: &FileInfo) -> String {
    let file_type = if info.is_dir {
        "Directory"
    } else if info.is_file {
        "File"
    } else {
        "Other"
    };

    let size = if info.is_dir {
        "—".to_string()
    } else {
        format_size(info.size)
    };

    let modified = info.modified
        .and_then(|t| t.elapsed().ok())
        .map(|elapsed| {
            let secs = elapsed.as_secs();
            if secs < 60 {
                format!("{} seconds ago", secs)
            } else if secs < 3600 {
                format!("{} minutes ago", secs / 60)
            } else if secs < 86400 {
                format!("{} hours ago", secs / 3600)
            } else {
                format!("{} days ago", secs / 86400)
            }
        })
        .unwrap_or_else(|| "Unknown".to_string());

    let permissions = if info.readonly { "Read-only" } else { "Read/Write" };

    format!(
        "Type: {}\nSize: {}\nModified: {}\nPermissions: {}",
        file_type, size, modified, permissions
    )
}

/// Generate preview using a backend
pub fn generate_preview_with_backend(backend: &Arc<dyn FilesystemBackend>, path: &str) -> String {
    let info = match backend.metadata(path) {
        Ok(info) => info,
        Err(e) => return format!("Error: {}", e),
    };

    let metadata_str = format_file_metadata_from_info(&info);

    if info.is_dir {
        match backend.list_dir(path) {
            Ok(entries) => {
                let mut result = metadata_str;
                result.push_str("\n\n=== Contents ===\n");
                let mut names: Vec<String> = entries.iter().map(|e| {
                    if e.info.is_dir {
                        format!("{}/", e.name)
                    } else {
                        e.name.clone()
                    }
                }).collect();
                names.sort();
                let total = names.len();
                for name in names.iter().take(20) {
                    result.push_str(&format!("  {}\n", name));
                }
                if total > 20 {
                    result.push_str(&format!("  ... and {} more items\n", total - 20));
                }
                result
            }
            Err(e) => format!("{}\n\n[Error reading directory: {}]", metadata_str, e),
        }
    } else if info.is_file {
        match backend.read_file(path, 64 * 1024) {
            Ok(data) => {
                match String::from_utf8(data) {
                    Ok(text) => {
                        let mut result = metadata_str;
                        result.push_str("\n\n=== Preview ===\n");
                        result.push_str(&text);
                        result
                    }
                    Err(_) => format!("{}\n\n[Binary file - cannot preview]", metadata_str),
                }
            }
            Err(e) => format!("{}\n\n[{}]", metadata_str, e),
        }
    } else {
        format!("{}\n\n[Special file type]", metadata_str)
    }
}

/// Formats file size in human-readable format
pub fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}


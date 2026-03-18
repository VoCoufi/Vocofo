use std::io::{Error, Result};
use std::path::{Path, PathBuf};
use std::{fs, io};

use crate::context::UiState::Normal;
use crate::context::{Context, PanelState};

/// Result type for file operations that can return any error type
pub type FileResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Reads the directory specified in `panel.path` and populates `panel.items`.
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

    for entry_result in fs::read_dir(&panel.path)? {
        let entry = entry_result?;
        let file_name = entry.file_name()
            .into_string()
            .map_err(|_| io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid UTF-8 in filename"
            ))?;

        let metadata = entry.metadata()?;

        if metadata.is_dir() {
            folders.push(format!("{}/", file_name));
        } else {
            files.push(file_name);
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

/// Deletes a file or directory at the specified path.
pub fn delete(path: impl AsRef<Path>) -> Result<()> {
    let path_str = path.as_ref().to_string_lossy();

    if path_str.contains("../") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Cannot delete parent directory"
        ));
    }

    let metadata = fs::metadata(&path)?;

    if metadata.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// Creates a directory and all its parent components if they are missing.
pub fn create_dir(path: impl AsRef<Path>) -> Result<()> {
    fs::create_dir_all(path)
}

/// Opens a file in the default editor
pub fn open_file(path: impl AsRef<Path>) -> FileResult<()> {
    edit::edit_file(path.as_ref())
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

/// Converts a relative path to an absolute canonical path
pub fn directory_path(folder_path: impl AsRef<Path>) -> FileResult<String> {
    let canonical_path = fs::canonicalize(folder_path)?;

    canonical_path.to_str()
        .ok_or_else(|| {
            Box::new(io::Error::new(
                io::ErrorKind::InvalidData,
                "Path contains invalid Unicode"
            )) as Box<dyn std::error::Error>
        })
        .map(|s| s.to_string())
}

/// Handle creating a new directory from the user input
pub fn handle_create_directory(context: &mut Context) -> FileResult<()> {
    let input = context.get_input()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("No input provided"))?;

    let path = PathBuf::from(&context.active().path).join(input);

    context.set_ui_state(Normal);
    create_dir(&path)?;
    context.set_input(String::default());
    context.active_mut().state = 0;

    Ok(())
}

/// Renames a file or directory
pub fn rename(old_path: impl AsRef<Path>, new_path: impl AsRef<Path>) -> Result<()> {
    let old = old_path.as_ref();
    let new = new_path.as_ref();

    if !old.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Source does not exist: {}", old.display()),
        ));
    }

    if new.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("Target already exists: {}", new.display()),
        ));
    }

    fs::rename(old, new)
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

    let old_path = PathBuf::from(&panel.path).join(&selected);
    let new_path = PathBuf::from(&panel.path).join(&new_name);

    context.set_ui_state(Normal);
    rename(&old_path, &new_path)?;
    context.set_input(String::default());

    Ok(())
}

/// Recursively copy a directory (contents) from `src` to `dst`.
pub fn copy_dir(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> FileResult<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();

    if !src.is_dir() {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Source is not a directory",
        )));
    }

    let src_canon = fs::canonicalize(src)?;
    let dst_canon_parent = match dst.parent() {
        Some(p) => fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf()),
        None => dst.to_path_buf(),
    };
    if dst_canon_parent.starts_with(&src_canon) && dst.file_name() == src.file_name() {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Destination is within the source directory",
        )));
    }

    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir(&from, &to)?;
        } else if file_type.is_file() {
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&from, &to)?;
        } else if file_type.is_symlink() {
            let target = fs::read_link(&from)?;
            let resolved = if target.is_absolute() {
                target
            } else {
                from.parent().unwrap_or(Path::new("")).join(target)
            };
            if resolved.is_dir() {
                copy_dir(&resolved, &to)?;
            } else {
                if let Some(parent) = to.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&resolved, &to)?;
            }
        }
    }

    Ok(())
}

/// Resolve source and destination paths for a paste operation
pub fn resolve_paste_paths(context: &mut Context) -> FileResult<(PathBuf, PathBuf)> {
    let from = PathBuf::from(context.get_copy_path());
    let dest_dir = resolve_path_from(context)?;
    let item_name = from.file_name()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Invalid source path"))?;
    let to = dest_dir.join(item_name);
    Ok((from, to))
}

fn resolve_path_from(context: &mut Context) -> FileResult<PathBuf> {
    let panel = context.active();
    let selected_item = panel.get_selected_item()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("No item selected"))?;
    let base_path = PathBuf::from(panel.path.clone());

    if panel.get_state() != 0 {
        let selected_item_metadata = fs::metadata(base_path.join(selected_item))
            .map_err(|e| {
                Box::new(Error::new(e.kind(), format!("Cannot access destination: {}", e)))
                    as Box<dyn std::error::Error>
            })?;

        if selected_item_metadata.is_dir() {
            return Ok(base_path.join(selected_item.trim_end_matches('/')));
        }
    }

    Ok(base_path)
}

/// Reads up to 64KB of a file for preview
pub fn read_file_preview(path: &Path) -> FileResult<String> {
    use std::io::Read;

    const MAX_PREVIEW_SIZE: usize = 64 * 1024;

    let mut file = fs::File::open(path)?;
    let mut buffer = vec![0u8; MAX_PREVIEW_SIZE];

    let bytes_read = file.read(&mut buffer)?;
    buffer.truncate(bytes_read);

    match String::from_utf8(buffer) {
        Ok(text) => Ok(text),
        Err(_) => Err(Box::new(Error::new(
            io::ErrorKind::InvalidData,
            "Binary file - cannot preview"
        )))
    }
}

/// Gets a preview of directory contents (first 20 items)
pub fn get_directory_preview(path: &Path) -> FileResult<Vec<String>> {
    const MAX_ITEMS: usize = 20;

    let mut folders = Vec::new();
    let mut files = Vec::new();
    let mut total_count = 0;

    for entry_result in fs::read_dir(path)? {
        let entry = entry_result?;
        total_count += 1;

        if folders.len() + files.len() < MAX_ITEMS {
            let file_name = entry.file_name()
                .into_string()
                .map_err(|_| Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid UTF-8 in filename"
                ))?;

            let metadata = entry.metadata()?;

            if metadata.is_dir() {
                folders.push(format!("{}/", file_name));
            } else {
                files.push(file_name);
            }
        }
    }

    folders.sort();
    files.sort();

    let mut result = folders;
    result.extend(files);

    if total_count > MAX_ITEMS {
        result.push(format!("... and {} more items", total_count - MAX_ITEMS));
    }

    Ok(result)
}

/// Formats file metadata for display
pub fn format_file_metadata(path: &Path) -> String {
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => return format!("Error reading metadata: {}", e),
    };

    let file_type = if metadata.is_dir() {
        "Directory"
    } else if metadata.is_file() {
        "File"
    } else {
        "Other"
    };

    let size = if metadata.is_dir() {
        match fs::read_dir(path) {
            Ok(entries) => {
                let count = entries.count();
                format!("{} items", count)
            }
            Err(_) => "Unknown".to_string(),
        }
    } else {
        format_size(metadata.len())
    };

    let modified = metadata.modified()
        .ok()
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

    let permissions = if metadata.permissions().readonly() {
        "Read-only"
    } else {
        "Read/Write"
    };

    format!(
        "Type: {}\nSize: {}\nModified: {}\nPermissions: {}",
        file_type, size, modified, permissions
    )
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

/// Main function to generate preview content for any path
pub fn generate_preview(path: &Path) -> String {
    let metadata_str = format_file_metadata(path);

    if !path.exists() {
        return format!("{}\n\n[File not found]", metadata_str);
    }

    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => return format!("{}\n\n[Error: {}]", metadata_str, e),
    };

    if metadata.is_dir() {
        match get_directory_preview(path) {
            Ok(items) => {
                let mut result = metadata_str;
                result.push_str("\n\n=== Contents ===\n");
                for item in items {
                    result.push_str(&format!("  {}\n", item));
                }
                result
            },
            Err(e) => format!("{}\n\n[Error reading directory: {}]", metadata_str, e),
        }
    } else if metadata.is_file() {
        match read_file_preview(path) {
            Ok(content) => {
                let mut result = metadata_str;
                result.push_str("\n\n=== Preview ===\n");
                result.push_str(&content);
                result
            },
            Err(e) => format!("{}\n\n[{}]", metadata_str, e),
        }
    } else {
        format!("{}\n\n[Special file type]", metadata_str)
    }
}

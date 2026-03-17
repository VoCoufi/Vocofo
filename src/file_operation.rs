use ratatui::{
    style::{Style, Stylize},
    widgets::ListItem,
};
use std::io::{Error, Result};
use std::path::{Path, PathBuf};
use std::{fs, io};

use crate::context::UiState::Normal;
use crate::context::Context;

/// Result type for file operations that can return any error type
pub type FileResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Generates a list of directory entries (folders and files) for display.
///
/// This function takes a mutable reference to the application context,
/// reads the directory specified in `context.path`, and populates both
/// `context.items` and a list of styled `ListItem` objects for rendering. 
/// The directory entries are categorized into folders and files, sorted
/// alphabetically, and styled differently for display:
///
/// - Folders are styled in blue and listed before files.
/// - Files are styled in green and listed after folders.
///
/// ### Parameters:
///
/// - `context`: A mutable reference to the `Context` object containing the
///    application state, including the current path to be listed and the `items`
///    list to be updated.
///
/// ### Returns:
///
/// On success, returns a `Result` containing a `Vec<ListItem<'static>>`, where:
/// - Each `ListItem` represents a directory entry styled appropriately for its type.
/// - Folders are listed with a trailing slash (e.g., `folder/`).
///
/// On failure, returns an `io::Error` wrapped in a `Result::Err`.
///
/// ### Behavior:
///
/// - Clears the current `context.items`.
/// - Reads and processes the directory entries in `context.path`.
/// - Adds `../` at the top of the folders list for navigation to the parent directory.
/// - Ensures valid UTF-8 conversion for file names.
/// - Sorts folders and files alphabetically.
/// - Adds styled folder and file entries to the `ListItem` output and the `context.items`.
///
/// ### Errors:
///
/// This function can return an error in the following situations:
/// - If the provided path in `context.path` is invalid or inaccessible.
/// - If the filenames in the directory contain invalid UTF-8 bytes.
/// - If an `io::Error` occurs while reading the directory or retrieving metadata.
///
/// ### Examples:
///
/// ```rust
/// let mut context = Context::new("/some/directory");
/// match list_children(&mut context) {
///     Ok(list) => {
///         for item in list {
///             println!("{}", item.content());
///         }
///     },
///     Err(err) => eprintln!("Error listing directory contents: {}", err),
/// }
/// ```
///
/// This will print the styled directory contents to the console with folders listed
/// in blue and files in green.
///
/// ### Dependencies:
///
/// This function relies on the following modules being in scope:
/// - `std::fs::read_dir` for reading directory entries.
/// - `std::io::Error` for handling I/O-related errors.
/// - Types such as `Style` and `ListItem` for styling and rendering directory entries.
pub fn list_children(context: &mut Context) -> Result<Vec<ListItem<'static>>> {
    let mut list = Vec::new();
    let mut folders = vec!["../".to_string()];
    let mut files = Vec::new();

    // Clear the current items list
    context.items.clear();

    // Read directory contents
    for entry_result in fs::read_dir(&context.path)? {
        let entry = entry_result?;
        let file_name = entry.file_name()
            .into_string()
            .map_err(|_| io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid UTF-8 in filename"
            ))?;

        let metadata = entry.metadata()?;

        // Categorize as folder or file
        if metadata.is_dir() {
            folders.push(format!("{}/", file_name));
        } else {
            files.push(file_name);
        }
    }

    // Sort entries alphabetically
    folders.sort();
    files.sort();

    // Add folders to the display list with blue styling
    for folder in &folders {
        context.items.push(folder.clone());
        list.push(ListItem::new(folder.clone()).style(Style::new().blue()));
    }

    // Add files to the display list with green styling
    for file in &files {
        context.items.push(file.clone());
        list.push(ListItem::new(file.clone()).style(Style::new().green()));
    }

    Ok(list)
}

/// Deletes a file or directory at the specified path.
///
/// # Parameters
/// - `path`: A path-like object (any type that implements `AsRef<Path>`) representing the file or directory to delete.
///
/// # Behavior
/// - If the path points to a file, the file is deleted.
/// - If the path points to a directory, the directory and all of its contents (including subdirectories) are deleted recursively.
/// - To prevent unintentional unsafe behavior, paths containing `"../"` (referring to parent directories) are disallowed. 
///   Attempting to delete such paths will result in an error.
///
/// # Return
/// - Returns `Ok(())` if the deletion is successful.
/// - Returns an `Err(io::Error)` if an error occurs during the process, such as if the path does not exist or 
///   the program lacks sufficient permissions to delete the file or directory.
///
/// # Errors
/// - Returns `io::ErrorKind::InvalidInput` if the provided path contains `"../"`, indicating an attempt to delete a parent directory.
/// - Returns any other relevant `io::Error` that might occur during file or directory removal operations, such as
///   `io::ErrorKind::NotFound`, `io::ErrorKind::PermissionDenied`, etc.
/// # Safety Notes
/// - This function ensures basic safety checks by prohibiting paths containing `"../"` to prevent accidental deletion
///   of unintended directories. Be cautious when specifying paths to avoid unintentional data loss.
pub fn delete(path: impl AsRef<Path>) -> Result<()> {
    let path_str = path.as_ref().to_string_lossy();

    // Safety check to prevent deleting parent directory
    if path_str.contains("../") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Cannot delete parent directory"
        ));
    }

    let metadata = fs::metadata(&path)?;

    // Choose appropriate deletion method based on file type
    if metadata.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// Creates a directory and all its parent components if they are missing.
///
/// This function ensures that the entire directory path specified by `path` exists,
/// creating intermediate directories as needed. If the directory already exists,
/// it does nothing and returns `Ok(())`.
///
/// # Arguments
///
/// * `path` - A path that implements the `AsRef<Path>` trait. This specifies the directory
///            path to be created.
///
/// # Returns
///
/// This function returns a `Result`:
/// * `Ok(())` - If the directory (and any necessary intermediate directories) is successfully created
///              or already exists.
/// * `Err(std::io::Error)` - If there is an I/O error while attempting to create the directory
///                            (e.g., insufficient permissions, invalid path).
///
pub fn create_dir(path: impl AsRef<Path>) -> Result<()> {
    fs::create_dir_all(path)
}

/// Opens a directory and updates the context accordingly.
///
/// # Arguments
/// * `context` - A mutable reference to a `Context` structure that will be updated by this function.
///
/// The function performs the following operations:
/// 1. Calls the `set_full_path` method on the provided `Context` to establish the full path.
/// 2. Resets the `state` field of the `Context` to `0`.
///
/// # Returns
/// * `FileResult<()>` - Returns `Ok(())` on successful execution, indicating the operation was completed successfully. Any potential errors will be encapsulated in the `FileResult` type.
pub fn open_dir(context: &mut Context) -> FileResult<()> {
    context.set_full_path();
    context.state = 0;
    Ok(())
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

/// Handle the delete operation for the currently selected item
pub fn handle_delete_operation(context: &mut Context) -> FileResult<()> {
    let selected_item = context.get_selected_item()
        .ok_or_else(|| Box::<dyn std::error::Error>::from(
            "No item selected"
        ))?;

    let path = PathBuf::from(&context.path).join(selected_item);

    delete(&path)?;

    // Update the selection state if necessary
    if context.state > 0 {
        context.decrease_state();
    }

    Ok(())
}

/// Handle creating a new directory from the user input
pub fn handle_create_directory(context: &mut Context) -> FileResult<()> {
    let input = context.get_input()
        .ok_or_else(|| Box::<dyn std::error::Error>::from(
            "No input provided"
        ))?;

    let path = PathBuf::from(&context.path).join(input);

    // Close the popup first
    context.set_ui_state(Normal);

    // Create the directory
    create_dir(&path)?;

    // Reset input and state
    context.set_input(String::default());
    context.state = 0;

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

    let selected = context.get_selected_item()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("No item selected"))?
        .clone();

    let old_path = PathBuf::from(&context.path).join(&selected);
    let new_path = PathBuf::from(&context.path).join(&new_name);

    context.set_ui_state(Normal);
    rename(&old_path, &new_path)?;
    context.set_input(String::default());

    Ok(())
}

/// Recursively copy a directory (contents) from `src` to `dst`.
/// - Creates `dst` if it doesn't exist.
/// - Skips `.` and `..`.
/// - Fails if `src` is not a directory.
/// NOTE: Does not preserve all metadata; extend if needed.
pub fn copy_dir(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> FileResult<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();

    if !src.is_dir() {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Source is not a directory",
        )));
    }

    // Prevent copying a directory into itself or its subdirectory
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
            // ensure a parent exists (it should but be safe)
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&from, &to)?;
        } else if file_type.is_symlink() {
            // Choose a policy: here we dereference and copy the target contents if file,
            // and recursively copy if directory symlink.

            //VOCO: maybe we should just copy the symlink?
            
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

/// Copy a file or directory:
/// - If `path_from` is a file, copies the file to `path_to` (creating parents).
/// - If `path_from` is a directory, performs a recursive copy to `path_to` (creating it).
pub fn copy_file(context: &mut Context) -> FileResult<()> {
    let from = PathBuf::from(&context.get_copy_path());
    let dest_dir = resolve_path_from(context)?;
    let item_name = from.file_name().ok_or_else(|| Error::new(io::ErrorKind::InvalidData, "Invalid source path"))?;
    let to = dest_dir.join(item_name);

    if from == to {
        return Err(Box::new(Error::new(
            io::ErrorKind::AlreadyExists,
            "Destination already exists",
        )));
    }

    let meta = fs::metadata(&from)?;
    if meta.is_dir() {
        let to = dest_dir.join(item_name);
        copy_dir(from, to)?;
        return Ok(());
    }

    let to = dest_dir.join(item_name);

    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }

    // Optional: prevent overwriting; adjust policy as needed.
    if to.exists() {
        return Err(Box::new(Error::new(
            io::ErrorKind::AlreadyExists,
            "Destination already exists",
        )));
    }

    fs::copy(from, to)?;
    Ok(())
}

fn resolve_path_from(context: &mut Context) -> FileResult<PathBuf> {
    let selected_item = context.get_selected_item()
        .ok_or_else(|| Box::<dyn std::error::Error>::from(
            "No item selected"
        ))?;
    let base_path = PathBuf::from(context.path.clone());

    if context.get_state() != 0 {
        let selected_item_metadata = fs::metadata(base_path.join(selected_item))
            .map_err(
                |e| {
                    Box::new(
                        Error::new(e.kind(), format!("Cannot access destination: {}", e)))
                }
                    as Box<dyn std::error::Error>)?;

        if selected_item_metadata.is_dir() {
            return Ok(base_path.join(selected_item.trim_end_matches('/')));
        }
    }

    Ok(base_path)
}

/// Reads up to 64KB of a file for preview
/// Returns error if file is binary (non-UTF8) or unreadable
pub fn read_file_preview(path: &Path) -> FileResult<String> {
    use std::io::Read;

    const MAX_PREVIEW_SIZE: usize = 64 * 1024; // 64KB

    let mut file = fs::File::open(path)?;
    let mut buffer = vec![0u8; MAX_PREVIEW_SIZE];

    let bytes_read = file.read(&mut buffer)?;
    buffer.truncate(bytes_read);

    // Try to convert to UTF-8
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

    // Add "... and N more" if truncated
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

    // For directories, count items instead of calculating recursive size
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
    // Get metadata header
    let metadata_str = format_file_metadata(path);

    // Check if path exists
    if !path.exists() {
        return format!("{}\n\n[File not found]", metadata_str);
    }

    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => return format!("{}\n\n[Error: {}]", metadata_str, e),
    };

    // Handle directories
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
    }
    // Handle files
    else if metadata.is_file() {
        match read_file_preview(path) {
            Ok(content) => {
                let mut result = metadata_str;
                result.push_str("\n\n=== Preview ===\n");
                result.push_str(&content);
                result
            },
            Err(e) => {
                // Binary file or read error
                format!("{}\n\n[{}]", metadata_str, e)
            }
        }
    }
    // Handle other types
    else {
        format!("{}\n\n[Special file type]", metadata_str)
    }
}

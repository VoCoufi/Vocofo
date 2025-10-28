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
            io::ErrorKind::InvalidInput,
            "Source and destination are the same",
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

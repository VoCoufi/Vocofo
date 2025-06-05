use std::{fs, io};
use std::io::Result;
use std::path::{Path, PathBuf};
use ratatui::{
    style::{Style, Stylize},
    widgets::ListItem,
};

use crate::context::Context;

/// Result type for file operations that can return any error type
pub type FileResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Lists all files and directories in the current context path
/// Returns a vector of styled ListItems for display
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

/// Deletes a file or directory at the specified path
/// Returns an error if trying to delete parent directory
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

/// Creates a new directory at the specified path
pub fn create_dir(path: impl AsRef<Path>) -> Result<()> {
    fs::create_dir_all(path)
}

/// Updates the context to navigate into the selected directory
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

    // Update selection state if necessary
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
    context.set_popup();

    // Create the directory
    create_dir(&path)?;

    // Reset input and state
    context.set_input(String::default());
    context.state = 0;

    Ok(())
}
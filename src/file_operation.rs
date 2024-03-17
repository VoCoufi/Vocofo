use std::fs;
use std::process::Command;
use std::{io::Result, str::FromStr};

use ratatui::{
    style::{Style, Stylize},
    widgets::ListItem,
};

use crate::context::Context;

pub fn list_children(context: &mut Context) -> Result<Vec<ListItem<'static>>> {
    let mut list = Vec::new();
    let mut folders: Vec<String> = Vec::new();
    folders.push("../".to_string());

    let mut files = Vec::new();

    context.items = Vec::new();

    

    for child in fs::read_dir(&context.path)? {
        let child = child?;
        let name: String = child.file_name().into_string().unwrap();
        let metadata = child.metadata()?;
        
        /// TODO resolve symlink
        if metadata.is_dir() {
            folders.push(name + "/");
        } else {
            files.push(name);
        }
    }

    folders.sort();


    files.sort();

    for folder in folders {
        context.items.push(folder.clone());
        list.push(ListItem::new(folder).style(Style::new().blue()));
    }

    for file in files {
        context.items.push(file.clone());
        list.push(ListItem::new(file).style(Style::new().green()));
    }


    Ok(list)
}

pub fn delete(path: String) -> Result<()> {
    // TODO check if it is a folder and if its a folder show popup window

    fs::remove_file(path)
}

pub fn create_dir(path: String) -> Result<()> {
    fs::create_dir(path)
}

pub fn open_dir(context: &mut Context) {
    context.set_full_path();
    context.state = 0;
}

pub fn open_file(path: &str) {
    let _ = edit::edit_file(path);
}

pub fn directory_path(folder_path: &str) -> String {
    return String::from_str(fs::canonicalize(folder_path).ok().unwrap().to_str().unwrap())
        .ok()
        .unwrap();
}

fn get_type_from_metadata(metadata: fs::Metadata) {
    
}

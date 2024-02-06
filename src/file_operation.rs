use std::{fs, io::Result};

use ratatui::{style::{Style, Stylize}, widgets::ListItem};

pub fn list_children() -> Result<Vec<ListItem<'static>>> {
    let mut list = Vec::new();
    let mut folders = Vec::new();
    let mut files = Vec::new();

    for child in fs::read_dir(".")? {
        let child = child?;
        let name: String = child.file_name().into_string().unwrap();

        if child.file_type()?.is_dir() {
            folders.push(name + "/");
        } else {
            files.push(name);
        }
    }

    folders.sort();
    files.sort();

    for folder in folders {
        list.push(ListItem::new(folder).style(Style::new().blue()));
    }

    for file in files {
        list.push(ListItem::new(file).style(Style::new().green()));
    }

    Ok(list)
}

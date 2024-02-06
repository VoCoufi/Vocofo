use std::{fs, io::Result};

use ratatui::widgets::ListItem;

pub fn list_children() -> Result<Vec<ListItem<'static>>> {
    let mut list = Vec::new();

    for child in fs::read_dir(".")? {
        let child = child?;
        let name: String = child.file_name().into_string().unwrap();

        list.push(ListItem::new(name));
    }

    Ok(list)
}

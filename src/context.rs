use std::fs;

use crate::file_operation;

pub struct Context {
    pub path: String,
    pub items: Vec<String>,
    pub state: usize,
    pub popup: bool
}

impl Context {
    pub fn new() -> Context {
        Context {
            path: file_operation::directory_path("."),
            items: Vec::new(),
            state: 0,
            popup: false,
        }
    }

    /// Returns the increment state of this [`Context`].
    pub fn increment_state(&mut self) {
        self.state += 1;
    }

    pub fn decrease_state(&mut self) {
        self.state -= 1;
    }

    pub fn get_selected_item(&self) -> Option<&String> {
        self.items.get(self.state)
    }

    pub fn get_popup(&self) -> Option<bool> {
        Some(self.popup)
    }

    pub fn set_popup(&mut self) {
        self.popup = !self.get_popup().unwrap()
    }

    pub fn set_full_path(&mut self) {
        let new_directory = self.path.clone() + "/" + self.get_selected_item().unwrap();
        self.path = file_operation::directory_path(&new_directory);
    }

    pub fn open_item(&mut self) {
        let file = fs::metadata(self.path.clone() + "/" + self.get_selected_item().unwrap()).unwrap();

        if file.is_dir() {
            self.set_full_path();
            self.state = 0;
        } else if file.is_file() {
            file_operation::open_file(&(self.path.clone() + "/" + self.get_selected_item().unwrap()))

            //let file_path = self.path.clone() + "/" + self.get_selected_item().unwrap();
            //let _ = edit::edit_file(file_path);
        }

    }
}
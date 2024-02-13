use std::fs;

use crate::file_operation;

pub struct Context {
    pub path: String,
    pub items: Vec<String>,
    pub state: usize
}

impl Context {
    pub fn new() -> Context {
        Context {
            path: file_operation::directory_path("."),
            items: Vec::new(),
            state: 0,
        }
    }

    pub fn increment_state(mut self) {
        self.state += 1;
    }

    pub fn decrease_state(mut self) {
        self.state -= 1;
    }

    pub fn get_selected_item(self) -> Option<String> {
        self.items.get(self.state).cloned()
    }
}
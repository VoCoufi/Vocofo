use crate::file_operation;
use std::fs;
use std::fs::Metadata;
use std::path::PathBuf;

/// A structure that holds the context and state information for a specific application or system.
/// It encapsulates various configurations, user inputs, and state-related flags.
///
/// # Fields
///
/// * `exit` - A boolean flag that indicates whether the application should exit or not.
/// * `path` - A `String` representing the current path or directory being used in the application.
/// * `items` - A `Vec<String>` storing a collection of items, which could represent entries, options, or other data.
/// * `state` - A `usize` representing the current state or index in the application, often used for navigation or tracking.
/// * `popup` - A boolean flag that determines whether a popup should be displayed or not.
/// * `confirm_popup` - A boolean flag to indicate if a confirmation popup is active or required.
/// * `confirm_popup_size` - A boolean flag that determines whether the size of the confirmation popup needs to be adjusted or checked.
/// * `input` - A `String` representing the user's input or a field for capturing user-typed text.
pub struct Context {
    pub exit: bool,
    pub path: String,
    pub items: Vec<String>,
    pub state: usize,
    pub ui_state: UiState,
    pub popup: bool,
    pub confirm_popup: bool,
    pub confirm_popup_size: bool,
    pub input: String,
    pub copy_path: String,
}

/// Represents different UI states
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum UiState {
    Normal,
    CreatePopup,
    ConfirmDelete,
}


impl Context {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            exit: false,
            path: file_operation::directory_path(".")?,
            items: Vec::new(),
            state: 0,
            ui_state: UiState::Normal,
            popup: false,
            confirm_popup: false,
            confirm_popup_size: false,
            input: String::default(),
            copy_path: String::default(),
        })
    }
    
    pub fn get_exit(&self) -> Option<bool> {
        Some(self.exit)
    }
    
    pub fn set_exit(&mut self) {
        self.exit = true;
    }

    /// Returns the increment state of this [`Context`].
    pub fn increment_state(&mut self) {
        self.state += 1;
    }

    pub fn decrease_state(&mut self) {
        if self.state > 0 {
            self.state -= 1;
        }
    }

    pub fn get_selected_item(&self) -> Option<&String> {
        self.items.get(self.state)
    }

    pub fn get_popup(&self) -> Option<bool> {
        Some(self.popup)
    }

    pub fn set_popup(&mut self) {
        let item = match self.get_popup() {
            Some(item) => item,
            None => return,
        };

        self.popup = !item
    }
    
    pub fn get_confirm_popup(&self) -> Option<bool> {
        Some(self.confirm_popup)
    }
    
    pub fn set_ui_state(&mut self, ui_state: UiState) {
        self.ui_state = ui_state;
    }
    
    pub fn get_ui_state(&self) -> Option<UiState> {
        Some(self.ui_state)
    }
    
    pub fn set_confirm_popup(&mut self) {
        let item = match self.get_confirm_popup() {
            Some(item) => item,
            None => return,
        };

        self.confirm_popup = !item
    }

    pub fn get_confirm_button_selected(&self) -> Option<bool> {
        // This is a placeholder - implement in your Context struct
        // For now, default to the safer option (No)
        Some(self.confirm_popup_size)
    }
    
    pub fn set_confirm_button_selected(&mut self) {
        let get_item = match self.get_confirm_button_selected() {
            Some(item) => item,
            None => return,
        };

        self.confirm_popup_size = !get_item
    }
    
    pub fn get_input(&self) -> Option<&String> {
        Some(&self.input)
    }
    
    pub fn set_input(&mut self, input: String) {
        self.input = input;
    }

    pub fn set_full_path(&mut self) {
        let get_item = match self.get_selected_item() {
            Some(item) => item,
            None => return,
        };

        let new_directory = PathBuf::from(self.path.clone()).join(get_item);
        let dir_path = match file_operation::directory_path(&new_directory) {
            Ok(path) => path,
            Err(err) => {
                //TODO error handling
                return;
            }
        };


        self.path = dir_path;
    }

    pub fn open_item(&mut self) {
        let file = match self.get_metadata_selected_item() {
            Some(file) => file,
            None => return,
        };

        if file.is_dir() {
            self.set_full_path();
            self.state = 0;
        } else if file.is_file() {
            let selected_item = match self.get_selected_item() {
                Some(item) => item,
                None => return,
            };

            let file_to_open = PathBuf::from(self.path.clone()).join(selected_item);

            match file_operation::open_file(&file_to_open) {
                Ok(_) => (),
                Err(err) => {
                    //TODO error handling
                }
            }

            //let file_path = self.path.clone() + "/" + self.get_selected_item().unwrap();
            //let _ = edit::edit_file(file_path);
        }
    }
    
    pub fn get_metadata_selected_item(&self) -> Option<Metadata> {
        let path = PathBuf::from(self.path.clone()).join(self.get_selected_item()?);
        let file = fs::metadata(path);

        file.ok()
    }

    pub fn get_copy_path(&self) -> &String {
        &self.copy_path
    }

    pub fn set_copy_path(&mut self) {
        let item = match self.get_selected_item() {
            Some(item) => item,
            None => return,
        };


        if item == "../" {
            return;
        }

        // Remove the trailing slash if it exists
        let clean_item = item.trim_end_matches("/");
        let path = PathBuf::from(&self.path);
        self.copy_path = path.join(clean_item).to_string_lossy().to_string();
    }
    
    pub fn get_state(&self) -> usize {
        self.state
    }
}
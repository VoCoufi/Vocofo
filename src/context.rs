use crate::file_operation;
use std::fs;
use std::fs::Metadata;

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
}

/// Represents different UI states
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum UiState {
    Normal,
    CreatePopup,
    ConfirmDelete,
}


impl Context {
    pub fn new() -> Self {
        Self {
            exit: false,
            path: file_operation::directory_path(".").expect("REASON"),
            items: Vec::new(),
            state: 0,
            ui_state: UiState::Normal,
            popup: false,
            confirm_popup: false,
            confirm_popup_size: false,
            input: String::default(),
        }
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
        self.confirm_popup = !self.get_confirm_popup().unwrap()
    }

    pub fn get_confirm_button_selected(&self) -> Option<bool> {
        // This is a placeholder - implement in your Context struct
        // For now, default to the safer option (No)
        Some(self.confirm_popup_size)
    }
    
    pub fn set_confirm_button_selected(&mut self) {
        self.confirm_popup_size = !self.get_confirm_button_selected().unwrap()
    }
    
    pub fn get_input(&self) -> Option<&String> {
        Some(&self.input)
    }
    
    pub fn set_input(&mut self, input: String) {
        self.input = input;
    }

    pub fn set_full_path(&mut self) {
        let new_directory = self.path.clone() + "/" + self.get_selected_item().unwrap();
        self.path = file_operation::directory_path(&new_directory).expect("REASON");
    }

    pub fn open_item(&mut self) {
        let file = self.get_metadata_selected_item().unwrap();

        if file.is_dir() {
            self.set_full_path();
            self.state = 0;
        } else if file.is_file() {
            file_operation::open_file(&(self.path.clone() + "/" + self.get_selected_item().unwrap())).expect("TODO: panic message");

            //let file_path = self.path.clone() + "/" + self.get_selected_item().unwrap();
            //let _ = edit::edit_file(file_path);
        }
    }
    
    pub fn get_metadata_selected_item(&self) -> Option<Metadata> {
        let file = fs::metadata(self.path.clone() + "/" + self.get_selected_item().unwrap());
        
        Some(file.unwrap())
    }
}
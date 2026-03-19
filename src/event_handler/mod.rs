mod clipboard;
mod command_palette;
mod connect;
mod main_handler;
mod popups;
mod settings;

pub use command_palette::{PALETTE_ACTIONS, handle_command_palette_event};
pub use connect::{
    handle_bookmark_list_event, handle_bookmark_name_event, handle_connect_dialog_event,
};
pub use main_handler::handle_main_event;
pub use popups::{
    handle_chmod_popup_event, handle_confirm_popup_event, handle_file_popup_event,
    handle_overwrite_popup_event, handle_popup_event, handle_rename_popup_event,
    handle_search_event,
};
pub use settings::handle_settings_event;

/// Result type for event handler operations
pub(crate) type EventResult = Result<(), Box<dyn std::error::Error>>;

use crate::backend::{FileInfo, FilesystemBackend};
use crate::background_op::FileOpResult;
use crate::config::Config;
use crate::file_operation;
use crate::local_backend::LocalBackend;
use std::collections::HashSet;
use std::sync::mpsc;
use std::sync::Arc;

/// State for a single directory panel
pub struct PanelState {
    pub path: String,
    pub backend: Arc<dyn FilesystemBackend>,
    pub items: Vec<String>,
    pub filtered_items: Vec<String>,
    pub filter: String,
    pub state: usize,
    pub items_dirty: bool,
    pub show_hidden: bool,
    pub selected: HashSet<String>,
    pub preview_content: Option<String>,
    pub preview_last_item: Option<String>,
    pub visible_rows: usize,
}

impl PanelState {
    pub fn new(path: String, backend: Arc<dyn FilesystemBackend>) -> Self {
        Self {
            path,
            backend,
            items: Vec::new(),
            filtered_items: Vec::new(),
            filter: String::new(),
            state: 0,
            items_dirty: true,
            show_hidden: false,
            selected: HashSet::new(),
            preview_content: None,
            preview_last_item: None,
            visible_rows: 20,
        }
    }

    pub fn increment_state(&mut self) {
        self.state = self.state.saturating_add(1);
    }

    pub fn decrease_state(&mut self) {
        if self.state > 0 {
            self.state -= 1;
        }
    }

    pub fn page_down(&mut self) {
        let max = self.filtered_items.len().saturating_sub(1);
        self.state = (self.state + self.visible_rows).min(max);
    }

    pub fn page_up(&mut self) {
        self.state = self.state.saturating_sub(self.visible_rows);
    }

    pub fn go_to_first(&mut self) {
        self.state = 0;
    }

    pub fn go_to_last(&mut self) {
        self.state = self.filtered_items.len().saturating_sub(1);
    }

    pub fn get_selected_item(&self) -> Option<&String> {
        self.filtered_items.get(self.state)
    }

    pub fn get_metadata_selected_item(&self) -> Option<FileInfo> {
        let item = self.get_selected_item()?;
        let full_path = self.backend.join_path(&self.path, item.trim_end_matches('/'));
        self.backend.metadata(&full_path).ok()
    }

    pub fn apply_filter(&mut self) {
        if self.filter.is_empty() {
            self.filtered_items = self.items.clone();
        } else {
            let filter_lower = self.filter.to_lowercase();
            self.filtered_items = self.items.iter()
                .filter(|item| {
                    *item == "../" || item.to_lowercase().contains(&filter_lower)
                })
                .cloned()
                .collect();
        }
        self.state = 0;
    }

    pub fn clear_filter(&mut self) {
        self.filter.clear();
        self.filtered_items = self.items.clone();
        self.state = 0;
    }

    pub fn set_full_path(&mut self) -> Option<String> {
        let get_item = match self.get_selected_item() {
            Some(item) => item.clone(),
            None => return None,
        };

        let new_directory = self.backend.join_path(&self.path, &get_item);
        match self.backend.canonicalize(&new_directory) {
            Ok(path) => {
                self.path = path;
                None
            }
            Err(err) => Some(format!("Cannot open directory: {}", err)),
        }
    }

    /// Navigate to the parent directory regardless of current selection
    pub fn navigate_to_parent(&mut self) -> Option<String> {
        let parent = self.backend.join_path(&self.path, "../");
        match self.backend.canonicalize(&parent) {
            Ok(path) => {
                self.path = path;
                self.state = 0;
                self.invalidate_directory_cache();
                None
            }
            Err(err) => Some(format!("Cannot open parent directory: {}", err)),
        }
    }

    pub fn open_item(&mut self) -> Option<String> {
        let info = match self.get_metadata_selected_item() {
            Some(info) => info,
            None => return None,
        };

        if info.is_dir {
            let err = self.set_full_path();
            self.state = 0;
            return err;
        } else if info.is_file {
            let selected_item = match self.get_selected_item() {
                Some(item) => item.clone(),
                None => return None,
            };

            let file_to_open = self.backend.join_path(&self.path, &selected_item);

            if let Err(err) = file_operation::open_file_with_backend(&self.backend, &file_to_open) {
                return Some(format!("Cannot open file: {}", err));
            }
        }
        None
    }

    pub fn update_preview(&mut self) {
        let selected_item = match self.get_selected_item() {
            Some(item) => item.clone(),
            None => {
                self.preview_content = None;
                self.preview_last_item = None;
                return;
            }
        };

        if let Some(ref last_item) = self.preview_last_item {
            if last_item == &selected_item && self.preview_content.is_some() {
                return;
            }
        }

        let full_path = if selected_item == "../" {
            self.path.clone()
        } else {
            self.backend.join_path(&self.path, &selected_item)
        };

        let preview = file_operation::generate_preview_with_backend(&self.backend, &full_path);
        self.preview_content = Some(preview);
        self.preview_last_item = Some(selected_item);
    }

    pub fn get_preview_content(&self) -> Option<&String> {
        self.preview_content.as_ref()
    }

    pub fn invalidate_directory_cache(&mut self) {
        self.items_dirty = true;
    }

    pub fn get_state(&self) -> usize {
        self.state
    }

    pub fn toggle_selection(&mut self) {
        if let Some(item) = self.get_selected_item() {
            if item == "../" {
                return;
            }
            let item = item.clone();
            if !self.selected.remove(&item) {
                self.selected.insert(item);
            }
        }
    }

    pub fn select_all(&mut self) {
        for item in &self.filtered_items {
            if item != "../" {
                self.selected.insert(item.clone());
            }
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    pub fn has_selection(&self) -> bool {
        !self.selected.is_empty()
    }

    pub fn get_selected_paths(&self) -> Vec<String> {
        self.selected.iter()
            .map(|name| self.backend.join_path(&self.path, name.trim_end_matches('/')))
            .collect()
    }
}

/// Represents clipboard mode for copy/cut operations
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ClipboardMode {
    Copy,
    Cut,
}

/// Represents different UI states
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum UiState {
    Normal,
    CreatePopup,
    CreateFilePopup,
    ConfirmDelete,
    RenamePopup,
    SearchMode,
    ConfirmOverwrite,
    ConnectDialog,
}

/// Connection protocol for remote backends
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionProtocol {
    Sftp,
    Ftp,
}

/// State for the connection dialog
#[derive(Debug, Clone)]
pub struct ConnectDialogState {
    pub protocol: ConnectionProtocol,
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub key_path: String,
    pub focused_field: usize,
    pub error_message: Option<String>,
}

impl ConnectDialogState {
    pub fn new() -> Self {
        Self {
            protocol: ConnectionProtocol::Sftp,
            host: String::new(),
            port: "22".to_string(),
            username: String::new(),
            password: String::new(),
            key_path: String::new(),
            focused_field: 1, // start on host field
            error_message: None,
        }
    }

    pub fn field_count(&self) -> usize {
        6 // protocol, host, port, username, password, key_path
    }

    pub fn active_field_mut(&mut self) -> &mut String {
        match self.focused_field {
            1 => &mut self.host,
            2 => &mut self.port,
            3 => &mut self.username,
            4 => &mut self.password,
            5 => &mut self.key_path,
            _ => &mut self.host,
        }
    }
}

/// Central application state
pub struct Context {
    pub exit: bool,
    pub panels: [PanelState; 2],
    pub active_panel: usize,
    pub show_preview: bool,
    pub ui_state: UiState,
    pub confirm_popup_size: bool,
    pub input: String,
    pub copy_path: String,
    pub copy_paths: Vec<String>,
    pub clipboard_mode: ClipboardMode,
    pub copy_source_backend: Option<Arc<dyn FilesystemBackend>>,
    pub pending_paste: Option<(String, String, bool)>,
    pub status_message: Option<String>,
    pub active_operation: Option<mpsc::Receiver<FileOpResult>>,
    pub operation_description: Option<String>,
    pub spinner_tick: u8,
    pub pending_g: bool,
    pub connect_dialog: Option<ConnectDialogState>,
}

impl Context {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_config(Config::default())
    }

    pub fn with_config(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let backend: Arc<dyn FilesystemBackend> = Arc::new(LocalBackend::new());
        let initial_path = backend.canonicalize(&config.general.default_path)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        let show_hidden = config.general.show_hidden;
        let mut panel0 = PanelState::new(initial_path.clone(), Arc::clone(&backend));
        let mut panel1 = PanelState::new(initial_path, Arc::clone(&backend));
        panel0.show_hidden = show_hidden;
        panel1.show_hidden = show_hidden;
        Ok(Self {
            exit: false,
            panels: [panel0, panel1],
            active_panel: 0,
            show_preview: config.general.show_preview_on_start,
            ui_state: UiState::Normal,
            confirm_popup_size: false,
            input: String::default(),
            copy_path: String::default(),
            copy_paths: Vec::new(),
            clipboard_mode: ClipboardMode::Copy,
            copy_source_backend: None,
            pending_paste: None,
            status_message: None,
            active_operation: None,
            operation_description: None,
            spinner_tick: 0,
            pending_g: false,
            connect_dialog: None,
        })
    }

    pub fn active(&self) -> &PanelState {
        &self.panels[self.active_panel]
    }

    pub fn active_mut(&mut self) -> &mut PanelState {
        &mut self.panels[self.active_panel]
    }

    pub fn invalidate_all_caches(&mut self) {
        self.panels[0].invalidate_directory_cache();
        self.panels[1].invalidate_directory_cache();
    }

    pub fn toggle_active_panel(&mut self) {
        self.active_panel = 1 - self.active_panel;
    }

    pub fn get_exit(&self) -> Option<bool> {
        Some(self.exit)
    }

    pub fn set_exit(&mut self) {
        self.exit = true;
    }

    pub fn set_ui_state(&mut self, ui_state: UiState) {
        self.ui_state = ui_state;
    }

    pub fn get_ui_state(&self) -> Option<UiState> {
        Some(self.ui_state)
    }

    pub fn get_confirm_button_selected(&self) -> Option<bool> {
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

    pub fn get_copy_path(&self) -> &String {
        &self.copy_path
    }

    pub fn set_copy_path(&mut self) {
        let item = match self.active().get_selected_item() {
            Some(item) => item,
            None => return,
        };

        if item == "../" {
            return;
        }

        let clean_item = item.trim_end_matches("/");
        self.copy_path = self.active().backend.join_path(&self.active().path, clean_item);
    }

    pub fn set_status_message(&mut self, message: &str) {
        self.status_message = Some(message.to_string());
    }

    pub fn get_status_message(&self) -> Option<&String> {
        self.status_message.as_ref()
    }

    pub fn clear_status_message(&mut self) {
        self.status_message = None;
    }

    pub fn is_operation_running(&self) -> bool {
        self.active_operation.is_some()
    }

    pub fn start_operation(&mut self, receiver: mpsc::Receiver<FileOpResult>, description: String) {
        self.active_operation = Some(receiver);
        self.operation_description = Some(description);
    }

    pub fn check_operation(&mut self) -> Option<FileOpResult> {
        let receiver = self.active_operation.as_ref()?;
        match receiver.try_recv() {
            Ok(result) => {
                self.active_operation = None;
                self.operation_description = None;
                Some(result)
            }
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                let desc = self.operation_description.take().unwrap_or_default();
                self.active_operation = None;
                Some(FileOpResult {
                    description: desc,
                    result: Err("Operation thread crashed".to_string()),
                    clear_clipboard: false,
                })
            }
        }
    }
}

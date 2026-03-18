use std::rc::Rc;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    prelude::*,
    style::{Color, Modifier, Style},
    widgets::*,
};

use crate::context::{Context, UiState};
use crate::messages_enum::MessageEnum;
use crate::render;

/// Result type for UI operations
type UiResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Main UI rendering function
pub fn ui(frame: &mut Frame, context: &mut Context) -> UiResult<()> {
    let main_layout = create_main_layout(frame.area());

    render_title_bar(frame, &main_layout[0]);
    render_status_bar(frame, &main_layout[2], context);

    let browser_layout = create_browser_layout(&main_layout[1]);

    render_directory_panels(frame, &browser_layout, context)?;

    render_popups(frame, context)?;

    Ok(())
}

/// Creates the main application layout with three vertical sections
fn create_main_layout(area: Rect) -> Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),    // Title bar
            Constraint::Min(0),       // Main content area
            Constraint::Length(1),    // Status bar
        ])
        .split(area)
}

/// Creates the file browser layout with two equal panels
/// Stacks vertically when terminal is taller than wide, horizontally otherwise
fn create_browser_layout(area: &Rect) -> Rc<[Rect]> {
    let direction = if area.height > area.width {
        Direction::Vertical
    } else {
        Direction::Horizontal
    };

    Layout::default()
        .direction(direction)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(*area)
}

/// Renders the application title bar
fn render_title_bar(frame: &mut Frame, area: &Rect) {
    let title_block = Block::default()
        .borders(Borders::NONE)
        .title(MessageEnum::AppTitle.as_str())
        .title_alignment(Alignment::Center)
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    frame.render_widget(title_block, *area);
}

/// Renders the status bar with spinner, status message, or keyboard shortcuts
fn render_status_bar(frame: &mut Frame, area: &Rect, context: &Context) {
    const SPINNER_FRAMES: &[char] = &['|', '/', '-', '\\'];

    let (text, style) = if context.is_operation_running() {
        let spinner = SPINNER_FRAMES[context.spinner_tick as usize % SPINNER_FRAMES.len()];
        let desc = context.operation_description.as_deref().unwrap_or("Working...");
        (format!("{} {}", spinner, desc), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    } else if let Some(message) = context.get_status_message() {
        (message.clone(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    } else {
        let clipboard = format_clipboard_indicator(context);
        let shortcuts = create_keyboard_shortcuts();
        if clipboard.is_empty() {
            (shortcuts, Style::default().fg(Color::White))
        } else {
            (format!("{} | {}", clipboard, shortcuts), Style::default().fg(Color::White))
        }
    };

    let status_bar = Paragraph::new(text)
        .style(style)
        .alignment(Alignment::Left);

    frame.render_widget(status_bar, *area);
}

/// Formats the clipboard indicator for the status bar
fn format_clipboard_indicator(context: &Context) -> String {
    let mode_label = match context.clipboard_mode {
        crate::context::ClipboardMode::Copy => "COPY",
        crate::context::ClipboardMode::Cut => "CUT",
    };

    if !context.copy_paths.is_empty() {
        format!("[{}: {} items]", mode_label, context.copy_paths.len())
    } else if !context.copy_path.is_empty() {
        let name = std::path::Path::new(&context.copy_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| context.copy_path.clone());
        format!("[{}: {}]", mode_label, name)
    } else {
        String::new()
    }
}

/// Creates a formatted string of keyboard shortcuts
fn create_keyboard_shortcuts() -> String {
    let shortcuts = [
        ("Q", "Quit"),
        ("Enter", "Open"),
        ("D", "Delete"),
        ("C", "Copy"),
        ("V", "Paste"),
        ("X", "Cut"),
        ("R", "Rename"),
        ("P", "New Folder"),
        ("N", "New File"),
        ("/", "Search"),
        ("=", "Sync Dir"),
        ("Tab", "Switch Panel"),
        ("Bksp", "Parent Dir"),
        (".", "Hidden"),
        ("F3", "Preview"),
    ];

    shortcuts.iter()
        .map(|(key, action)| format!("[{}] {}", key, action))
        .collect::<Vec<_>>()
        .join(" | ")
}

/// Renders the directory browser panels
fn render_directory_panels(frame: &mut Frame, layout: &[Rect], context: &mut Context) -> UiResult<()> {
    if context.show_preview {
        // Preview mode: active panel + preview on the opposite side
        let active_idx = context.active_panel;
        let (panel_area, preview_area) = if active_idx == 0 {
            (layout[0], layout[1])
        } else {
            (layout[1], layout[0])
        };

        let is_searching = context.ui_state == UiState::SearchMode;
        render::render_panel(frame, panel_area, context.active_mut(), true, is_searching)?;
        render_preview_panel(frame, &preview_area, context)?;
    } else {
        // Dual panel mode: both panels visible
        let active = context.active_panel;
        let is_searching = context.ui_state == UiState::SearchMode;
        render::render_panel(frame, layout[0], &mut context.panels[0], active == 0, is_searching && active == 0)?;
        render::render_panel(frame, layout[1], &mut context.panels[1], active == 1, is_searching && active == 1)?;
    }

    Ok(())
}

/// Renders a preview panel for the selected item
fn render_preview_panel(frame: &mut Frame, area: &Rect, context: &mut Context) -> UiResult<()> {
    let selected_item = context.active().get_selected_item()
        .map(|s| s.as_str())
        .unwrap_or("[Nothing selected]");

    let preview_content = context.active().get_preview_content()
        .map(|s| s.as_str())
        .unwrap_or("No preview available");

    let inner_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let preview_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue))
        .title(format!(" Preview: {} ", selected_item))
        .title_alignment(Alignment::Center);

    let preview_paragraph = Paragraph::new(preview_content)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: false })
        .scroll((0, 0));

    frame.render_widget(preview_block, *area);
    frame.render_widget(preview_paragraph, inner_area);

    Ok(())
}

/// Renders the appropriate popup UI based on the current UI state.
fn render_popups(frame: &mut Frame, context: &mut Context) -> UiResult<()> {
    match context.get_ui_state().ok_or("UI state not available")? {
        UiState::ConfirmDelete => render::popup_confirm_delete(frame, context)?,
        UiState::CreatePopup => render::popup_name_creation(frame, context)?,
        UiState::CreateFilePopup => render::popup_create_file(frame, context)?,
        UiState::RenamePopup => render::popup_rename(frame, context)?,
        UiState::ConfirmOverwrite => render::popup_confirm_overwrite(frame, context)?,
        UiState::SearchMode | UiState::Normal => ()
    }

    Ok(())
}

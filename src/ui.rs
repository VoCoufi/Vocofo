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
fn create_browser_layout(area: &Rect) -> Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Horizontal)
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

/// Renders the status bar with either a status message or keyboard shortcuts
fn render_status_bar(frame: &mut Frame, area: &Rect, context: &Context) {
    let (text, style) = if let Some(message) = context.get_status_message() {
        (message.clone(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
    } else {
        (create_keyboard_shortcuts(), Style::default().fg(Color::White))
    };

    let status_bar = Paragraph::new(text)
        .style(style)
        .alignment(Alignment::Left);

    frame.render_widget(status_bar, *area);
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
        ("Tab", "Switch Panel"),
        ("Bksp", "Parent Dir"),
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

        render::render_panel(frame, panel_area, context.active_mut(), true)?;
        render_preview_panel(frame, &preview_area, context)?;
    } else {
        // Dual panel mode: both panels visible
        let active = context.active_panel;
        render::render_panel(frame, layout[0], &mut context.panels[0], active == 0)?;
        render::render_panel(frame, layout[1], &mut context.panels[1], active == 1)?;
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
        UiState::RenamePopup => render::popup_rename(frame, context)?,
        UiState::Normal => ()
    }

    Ok(())
}

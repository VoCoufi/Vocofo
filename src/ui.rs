use std::rc::Rc;
use ratatui::{
    prelude::*,
    widgets::*,
    layout::{Constraint, Direction, Layout, Alignment},
    style::{Color, Modifier, Style},
};

use crate::context::Context;
use crate::render;

/// Result type for UI operations
type UiResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Main UI rendering function
pub fn ui(frame: &mut Frame, context: &mut Context) -> UiResult<()> {
    // Create the main application layout
    let main_layout = create_main_layout(frame.area());

    // Render the application components
    render_title_bar(frame, &main_layout[0]);
    render_status_bar(frame, &main_layout[2]);

    // Create the file browser layout
    let browser_layout = create_browser_layout(&main_layout[1]);

    // Render the file browser components
    render_directory_panels(frame, &browser_layout, context)?;

    // Render any active popups on top
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
            Constraint::Percentage(50),  // Left panel
            Constraint::Percentage(50),  // Right panel
        ])
        .split(*area)
}

/// Renders the application title bar
fn render_title_bar(frame: &mut Frame, area: &Rect) {
    let title_block = Block::default()
        .borders(Borders::NONE)
        .title("Vocofo File Manager")
        .title_alignment(Alignment::Center)
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    frame.render_widget(title_block, *area);
}

/// Renders the status bar with keyboard shortcuts
fn render_status_bar(frame: &mut Frame, area: &Rect) {
    let shortcuts = create_keyboard_shortcuts();

    let status_bar = Paragraph::new(shortcuts)
        .style(Style::default().fg(Color::White))
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
        ("R", "Rename"),
        ("P", "New Folder"),
        ("Tab", "Switch Panel"),
    ];

    shortcuts.iter()
        .map(|(key, action)| format!("[{}] {}", key, action))
        .collect::<Vec<_>>()
        .join(" | ")
}

/// Renders the directory browser panels
fn render_directory_panels(frame: &mut Frame, layout: &[Rect], context: &mut Context) -> UiResult<()> {
    // Render the left directory panel
    render::render_left_directory(frame, Rc::new(layout.to_vec()), context)?;

    // Render the preview panel for the selected item
    render_preview_panel(frame, &layout[1], context)?;

    // Render the right directory panel if needed
    // render::render_right_directory(frame, Rc::new(layout.to_vec()), context)?;

    Ok(())
}

/// Renders a preview panel for the selected item
fn render_preview_panel(frame: &mut Frame, area: &Rect, context: &mut Context) -> UiResult<()> {
    // Get the selected item name or default to empty string
    let selected_item = context.get_selected_item().unwrap();

    // Create a styled block for the preview
    let preview_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue))
        .title(format!(" Preview: {} ", selected_item))
        .title_alignment(Alignment::Center);

    // Render the preview block
    frame.render_widget(preview_block, *area);

    // TODO: Add file preview content based on the selected item type
    // For example: text preview for text files, image info for images, etc.

    Ok(())
}

/// Renders active popups if any are visible
fn render_popups(frame: &mut Frame, context: &mut Context) -> UiResult<()> {
    // Check if the creation folder popup should be displayed
    if context.get_popup().expect("REASON") {
        render::popup_name_creation(frame, context)?;
    }

    // Check if the confirmation popup should be displayed
    if context.get_confirm_popup().expect("REASON") {
        render::popup_confirm_delete(frame, context)?;
    }

    Ok(())
}
use std::rc::Rc;
use ratatui::{
    prelude::*,
    widgets::*,
    layout::{Constraint, Direction, Layout, Alignment},
};


use crate::file_operation;
use crate::context::Context;

/// Error type for rendering operations
type RenderResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Renders the left directory panel with file/folder list
pub fn render_left_directory(frame: &mut Frame, inner_layout: Rc<Vec<Rect>>, context: &mut Context) -> RenderResult<()> {
    // Get the list of files and folders from the current path
    let items = file_operation::list_children(context)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    // Create a styled list widget
    let list = create_directory_list(&context.path, items, true);

    // Create a stateful list with current selection
    let mut state = ListState::default().with_selected(Some(context.state));

    // Render the widget in the left panel
    frame.render_stateful_widget(list, inner_layout[0], &mut state);

    Ok(())
}

/// Renders the right directory panel with file/folder list
pub fn render_right_directory(frame: &mut Frame, inner_layout: Rc<[Rect]>, context: &mut Context) -> RenderResult<()> {
    // Get the list of files and folders from the current path
    let items = file_operation::list_children(context)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    // Create a styled list widget
    let list = create_directory_list(&context.path, items, false);

    // Create a stateful list with current selection
    let mut state = ListState::default().with_selected(Some(context.state));

    // Render the widget in the right panel
    frame.render_stateful_widget(list, inner_layout[1], &mut state);

    Ok(())
}

/// Creates a styled directory list widget
fn create_directory_list<'a>(path: &str, items: Vec<ListItem<'a>>, is_primary: bool) -> List<'a> {
    let list = List::new(items)
        .block(
            Block::default()
                .title(path.to_string())
                .borders(Borders::ALL)
        )
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true)
        .highlight_spacing(HighlightSpacing::WhenSelected)
        .direction(ListDirection::TopToBottom);

    // Add different styling for primary vs secondary list if needed
    if !is_primary {
        // For example, we could add different styling here
    }

    list
}

/// Renders a popup window for creating a new folder
pub fn popup_window(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    // Get the input text or provide a default empty string
    let input_text = context.get_input().unwrap();

    // Create a paragraph widget with the input text
    let block = Block::default()
        .title("Create folder")
        .borders(Borders::ALL);

    let para = Paragraph::new(input_text.clone()).block(block);

    // Calculate the centered rectangle for the popup
    let area = centered_rect(frame.area());

    // Clear the background and render the popup
    frame.render_widget(Clear, area);
    frame.render_widget(para, area);

    Ok(())
}

/// Helper function to create a centered rect for input popup
/// Returns a rectangle positioned at the top center of the screen
fn centered_rect(r: Rect) -> Rect {
    // Create a vertical layout with specific heights
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),    // Top margin
            Constraint::Length(5),    // Popup height
            Constraint::Min(0),       // Remaining space
        ])
        .split(r);

    // Create a horizontal layout to center the popup
    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),  // Left margin
            Constraint::Percentage(60),  // Popup width
            Constraint::Percentage(20),  // Right margin
        ])
        .split(vertical_chunks[1]);

    // Return the centered rectangle
    horizontal_chunks[1]
}

/// Helper function to create a centered rect for confirmation popup
/// Returns a rectangle positioned at the center of the screen
fn centered_rect_confirm(r: Rect) -> Rect {
    // Create a vertical layout that divides the screen into three sections
    // and returns the middle section
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),  // Top margin
            Constraint::Percentage(40),  // Popup height
            Constraint::Percentage(30),  // Bottom margin
        ])
        .split(r);

    // Create a horizontal layout to center the popup
    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),  // Left margin
            Constraint::Percentage(60),  // Popup width
            Constraint::Percentage(20),  // Right margin
        ])
        .split(vertical_chunks[1]);

    // Return the centered rectangle
    horizontal_chunks[1]
}

/// Creates a styled button with minimum width and optional selected state
fn create_sized_button(text: &str, selected: bool, min_width: u16) -> Paragraph {
    let style = if selected {
        Style::default().bg(Color::Blue).fg(Color::Black)
    } else {
        Style::default().fg(Color::Blue)
    };

    let border_style = if selected {
        Style::default().fg(Color::Black)
    } else {
        Style::default().fg(Color::Blue)
    };

    // Add some padding around the text
    let display_text = format!(" {} ", text);

    Paragraph::new(display_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title_alignment(Alignment::Center)
                .padding(Padding::new(1, 1, 0, 0)) // Add horizontal padding
        )
        .alignment(Alignment::Center)
        .style(style)
}

/// Helper function to create a centered rectangle with specified percentage width and height
/// and ensure there's enough space for buttons
fn centered_rect_dialog(r: Rect, percent_width: u16, percent_height: u16) -> Rect {
    // Ensure percentages are within valid range
    let width_percent = percent_width.clamp(10, 90);
    let height_percent = percent_height.clamp(10, 90);

    // Calculate the width and height in absolute terms
    let width = (r.width * percent_width / 100).min(60);

    
    // Ensure a minimum height of 5 rows
    let calculated_height = r.height * height_percent / 100;
    let height = calculated_height.clamp(10, 16);  // Min 10, max 16 rows

    // Calculate the starting positions to center the rectangle
    let x = (r.width - width) / 2;
    let y = (r.height - height) / 2;

    // Create the rectangle
    Rect::new(
        r.x + x,
        r.y + y,
        width,
        height
    )
}

/// Renders an enhanced confirmation popup for deletion with properly sized buttons
pub fn popup_confirm_delete(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    // Get the selected item or provide a default
    let selected_item = context.get_selected_item()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("No item selected"))?;

    // Calculate the centered area for the popup with adequate size
    let area = centered_rect_dialog(frame.size(), 80, 10);

    // Create the outer block for the entire dialog
    let dialog_block = Block::default()
        .title(" Confirm Deletion ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Red));

    // Render the outer block first
    frame.render_widget(Clear, area);
    frame.render_widget(dialog_block.clone(), area);

    // Create the inner area within the block borders
    let inner_area = dialog_block.inner(area);

    // Split the inner area into sections with proper spacing
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(2),   // Warning icon/title area
            Constraint::Length(0),   // Spacer
            Constraint::Length(1),   // Message area
            Constraint::Length(1),   // Spacer
            Constraint::Length(3),   // Buttons area
        ])
        .split(inner_area);

    // Create a warning title with icon
    let warning_text = "⚠️  Warning: This action cannot be undone!";
    let warning = Paragraph::new(warning_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    // Create the main confirmation message
    let message = format!("Are you sure you want to delete \"{}\"?", selected_item);
    let message_paragraph = Paragraph::new(message)
        .alignment(Alignment::Center)
        .style(Style::default());

    // Create the button area with a horizontal layout and proper spacing
    let button_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(10),  // Left margin
            Constraint::Percentage(35),  // Yes button
            Constraint::Percentage(10),  // Space between buttons
            Constraint::Percentage(35),  // No button
            Constraint::Percentage(10),  // Right margin
        ])
        .split(chunks[4]);

    // Determine which button is currently selected
    let is_yes_selected = context.get_confirm_button_selected();

    // Create sized buttons that will properly display their text
    let yes_button = create_sized_button("Yes", is_yes_selected.unwrap(), 16);
    let no_button = create_sized_button("No", !is_yes_selected.unwrap(), 16);

    // Render all components
    frame.render_widget(warning, chunks[0]);
    frame.render_widget(message_paragraph, chunks[2]);
    frame.render_widget(yes_button, button_chunks[1]);
    frame.render_widget(no_button, button_chunks[3]);

    Ok(())
}
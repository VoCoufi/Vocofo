use ratatui::{
    prelude::*,
    style::{Style, Stylize},
    widgets::*,
    layout::{Constraint, Direction, Layout, Alignment},
};

use crate::file_operation;
use crate::context::PanelState;
use crate::context::Context;

/// Error type for rendering operations
type RenderResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Renders a directory panel, refreshing the item list only when dirty
pub fn render_panel(frame: &mut Frame, area: Rect, panel: &mut PanelState, is_active: bool, is_searching: bool) -> RenderResult<()> {
    if panel.items_dirty {
        file_operation::list_children(panel)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    }

    // Build styled ListItems with file details
    let panel_width = area.width.saturating_sub(4) as usize; // account for borders + highlight
    let items: Vec<ListItem> = panel.filtered_items.iter().map(|name| {
        if name == "../" {
            return ListItem::new(name.clone()).style(Style::new().blue());
        }

        let full_path = std::path::PathBuf::from(&panel.path).join(name.trim_end_matches('/'));
        let details = file_operation::format_item_details(&full_path);

        let is_selected = panel.selected.contains(name);
        let name_style = if is_selected {
            Style::new().yellow().bold()
        } else if name.ends_with('/') {
            Style::new().blue()
        } else {
            Style::new().green()
        };

        if details.is_empty() || panel_width < name.len() + details.len() + 2 {
            return ListItem::new(name.clone()).style(name_style);
        }

        let padding = panel_width.saturating_sub(name.len() + details.len());
        let line = Line::from(vec![
            Span::styled(name.clone(), name_style),
            Span::raw(" ".repeat(padding)),
            Span::styled(details, Style::new().dark_gray()),
        ]);
        ListItem::new(line)
    }).collect();

    let title = if is_searching {
        format!("{} [/: {}]", panel.path, panel.filter)
    } else if !panel.filter.is_empty() {
        format!("{} [filter: {}]", panel.path, panel.filter)
    } else {
        panel.path.clone()
    };

    let list = create_directory_list(&title, items, is_active);

    let mut state = ListState::default().with_selected(Some(panel.state));

    frame.render_stateful_widget(list, area, &mut state);

    Ok(())
}

/// Renders an enhanced confirmation popup for deletion with properly sized buttons
pub fn popup_confirm_delete(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    let selected_item = context.active().get_selected_item()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("No item selected"))?;

    let area = centered_rect_dialog(frame.area(), 80, 10);

    let dialog_block = Block::default()
        .title(" Confirm Deletion ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Red));

    frame.render_widget(Clear, area);
    frame.render_widget(dialog_block.clone(), area);

    let inner_area = dialog_block.inner(area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(0),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(inner_area);

    let warning_text = "⚠️ Warning: This action cannot be undone!";
    let warning = Paragraph::new(warning_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let message = format!("Are you sure you want to delete \"{}\"?", selected_item);
    let message_paragraph = Paragraph::new(message)
        .alignment(Alignment::Center)
        .style(Style::default());

    let button_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(35),
            Constraint::Percentage(10),
            Constraint::Percentage(35),
            Constraint::Percentage(10),
        ])
        .split(chunks[4]);

    let is_yes_selected = context.get_confirm_button_selected();

    let selected = is_yes_selected.unwrap_or(false);
    let yes_button = create_sized_button("Yes", selected);
    let no_button = create_sized_button("No", !selected);

    frame.render_widget(warning, chunks[0]);
    frame.render_widget(message_paragraph, chunks[2]);
    frame.render_widget(yes_button, button_chunks[1]);
    frame.render_widget(no_button, button_chunks[3]);

    Ok(())
}

pub fn popup_name_creation(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    let area = centered_rect_dialog(frame.area(), 80, 10);

    let dialog_block = Block::default()
        .title(" Create folder ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Red));

    frame.render_widget(Clear, area);
    frame.render_widget(dialog_block.clone(), area);

    let inner_area = dialog_block.inner(area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(inner_area);

    let warning_text = "Write the name of the folder.";
    let warning = Paragraph::new(warning_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let empty = String::new();
    let input_text = context.get_input().unwrap_or(&empty);
    let para = Paragraph::new(input_text.clone())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Black))
                .padding(Padding::new(1, 1, 0, 0))
        )
        .alignment(Alignment::Left)
        .style(Style::default().bg(Color::Blue).fg(Color::Black));

    frame.render_widget(warning, chunks[1]);
    frame.render_widget(para, chunks[3]);

    Ok(())
}

/// Renders a rename popup
pub fn popup_rename(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    let area = centered_rect_dialog(frame.area(), 80, 10);

    let dialog_block = Block::default()
        .title(" Rename ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(Clear, area);
    frame.render_widget(dialog_block.clone(), area);

    let inner_area = dialog_block.inner(area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(inner_area);

    let label = Paragraph::new("Enter new name:")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let empty = String::new();
    let input_text = context.get_input().unwrap_or(&empty);
    let para = Paragraph::new(input_text.clone())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Black))
                .padding(Padding::new(1, 1, 0, 0))
        )
        .alignment(Alignment::Left)
        .style(Style::default().bg(Color::Blue).fg(Color::Black));

    frame.render_widget(label, chunks[1]);
    frame.render_widget(para, chunks[3]);

    Ok(())
}

/// Renders an overwrite confirmation popup
pub fn popup_confirm_overwrite(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    let file_name = context.pending_paste.as_ref()
        .and_then(|(_, to, _)| to.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());

    let area = centered_rect_dialog(frame.area(), 80, 10);

    let dialog_block = Block::default()
        .title(" Confirm Overwrite ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));

    frame.render_widget(Clear, area);
    frame.render_widget(dialog_block.clone(), area);

    let inner_area = dialog_block.inner(area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(0),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(inner_area);

    let warning = Paragraph::new("⚠️ File already exists!")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let message = Paragraph::new(format!("Overwrite \"{}\"?", file_name))
        .alignment(Alignment::Center)
        .style(Style::default());

    let button_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(35),
            Constraint::Percentage(10),
            Constraint::Percentage(35),
            Constraint::Percentage(10),
        ])
        .split(chunks[4]);

    let selected = context.get_confirm_button_selected().unwrap_or(false);
    let yes_button = create_sized_button("Yes", selected);
    let no_button = create_sized_button("No", !selected);

    frame.render_widget(warning, chunks[0]);
    frame.render_widget(message, chunks[2]);
    frame.render_widget(yes_button, button_chunks[1]);
    frame.render_widget(no_button, button_chunks[3]);

    Ok(())
}

/// Creates a styled directory list widget
fn create_directory_list<'a>(path: &str, items: Vec<ListItem<'a>>, is_active: bool) -> List<'a> {
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    List::new(items)
        .block(
            Block::default()
                .title(path.to_string())
                .borders(Borders::ALL)
                .border_style(border_style)
        )
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        .highlight_symbol(">>")
        .repeat_highlight_symbol(true)
        .highlight_spacing(HighlightSpacing::WhenSelected)
        .direction(ListDirection::TopToBottom)
}

/// Creates a styled button with minimum width and optional selected state
fn create_sized_button(text: &str, selected: bool) -> Paragraph<'_> {
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

    let display_text = format!(" {} ", text);

    Paragraph::new(display_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title_alignment(Alignment::Center)
                .padding(Padding::new(1, 1, 0, 0))
        )
        .alignment(Alignment::Center)
        .style(style)
}

/// Helper function to create a centered rectangle
fn centered_rect_dialog(r: Rect, percent_width: u16, percent_height: u16) -> Rect {
    let height_percent = percent_height.clamp(10, 90);
    let width = (r.width * percent_width / 100).min(60);
    let calculated_height = r.height * height_percent / 100;
    let height = calculated_height.clamp(10, 16);
    let x = (r.width - width) / 2;
    let y = (r.height - height) / 2;

    Rect::new(r.x + x, r.y + y, width, height)
}

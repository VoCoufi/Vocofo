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

        let full_path = panel.backend.join_path(&panel.path, name.trim_end_matches('/'));
        let details = match panel.backend.metadata(&full_path) {
            Ok(info) => file_operation::format_item_details_from_info(&info),
            Err(_) => String::new(),
        };

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

    // Store visible rows for PageUp/PageDown calculations
    panel.visible_rows = area.height.saturating_sub(2) as usize;

    let position = if panel.filtered_items.is_empty() {
        " [0/0]".to_string()
    } else {
        format!(" [{}/{}]", panel.state + 1, panel.filtered_items.len())
    };

    let title = if is_searching {
        format!("{} [/: {}]{}", panel.path, panel.filter, position)
    } else if !panel.filter.is_empty() {
        format!("{} [filter: {}]{}", panel.path, panel.filter, position)
    } else {
        format!("{}{}", panel.path, position)
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

pub fn popup_create_file(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    let area = centered_rect_dialog(frame.area(), 80, 10);

    let dialog_block = Block::default()
        .title(" Create file ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Green));

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

    let label = Paragraph::new("Write the name of the file.")
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

/// Renders the chmod popup
pub fn popup_chmod(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    let area = centered_rect_dialog(frame.area(), 50, 8);

    let dialog_block = Block::default()
        .title(" chmod ")
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
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(inner_area);

    let label = Paragraph::new("Enter octal mode (e.g. 755):")
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
        .and_then(|(_, to, _)| {
            std::path::Path::new(to).file_name()
                .map(|n| n.to_string_lossy().to_string())
        })
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

/// Renders the bookmark list popup
pub fn popup_bookmark_list(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    let connections = &context.config.connections;
    let height = (connections.len() as u16 + 4).min(20);
    let area = centered_rect_dialog(frame.area(), 60, height);

    let dialog_block = Block::default()
        .title(" Bookmarks ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(Clear, area);
    frame.render_widget(dialog_block.clone(), area);

    let inner_area = dialog_block.inner(area);

    let items: Vec<Line> = connections.iter().enumerate().map(|(i, profile)| {
        let text = format!(
            " {} ({}://{}@{}:{}) ",
            profile.name, profile.protocol, profile.username, profile.host, profile.port
        );
        let style = if i == context.bookmark_selected {
            Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        Line::from(text).style(style)
    }).collect();

    let list = Paragraph::new(items);
    frame.render_widget(list, inner_area);

    Ok(())
}

/// Renders the bookmark name input popup
pub fn popup_bookmark_name(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    let area = centered_rect_dialog(frame.area(), 50, 8);

    let dialog_block = Block::default()
        .title(" Save Bookmark ")
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
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(inner_area);

    let label = Paragraph::new("Bookmark name:")
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

/// Renders the settings popup
pub fn popup_settings(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    let area = centered_rect_dialog(frame.area(), 55, 14);

    let dialog_block = Block::default()
        .title(" Settings ")
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
            Constraint::Length(1), // spacing
            Constraint::Length(1), // Panel Layout
            Constraint::Length(1), // spacing
            Constraint::Length(1), // Show Hidden
            Constraint::Length(1), // spacing
            Constraint::Length(1), // Show Preview
            Constraint::Length(1), // spacing
            Constraint::Length(1), // Default Path
            Constraint::Length(1), // spacing
            Constraint::Min(1),   // hint
        ])
        .split(inner_area);

    let state = context.settings_state.as_ref();
    let focused = state.map(|s| s.focused_field).unwrap_or(0);
    let editing_path = state.map(|s| s.editing_path).unwrap_or(false);

    let field_style = |idx: usize| -> Style {
        if idx == focused {
            Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        }
    };

    // Field 0: Panel Layout
    let layout_val = context.config.general.panel_layout.as_str();
    let layout_line = Line::from(vec![
        Span::styled("  Panel Layout   ", Style::default().fg(Color::Yellow)),
        Span::styled(format!("◂ {:^12} ▸", layout_val), field_style(0)),
    ]);
    frame.render_widget(Paragraph::new(layout_line), chunks[1]);

    // Field 1: Show Hidden
    let hidden_val = if context.config.general.show_hidden { " On " } else { " Off" };
    let hidden_line = Line::from(vec![
        Span::styled("  Show Hidden    ", Style::default().fg(Color::Yellow)),
        Span::styled(format!("  {:^12}  ", hidden_val), field_style(1)),
    ]);
    frame.render_widget(Paragraph::new(hidden_line), chunks[3]);

    // Field 2: Show Preview
    let preview_val = if context.config.general.show_preview_on_start { " On " } else { " Off" };
    let preview_line = Line::from(vec![
        Span::styled("  Show Preview   ", Style::default().fg(Color::Yellow)),
        Span::styled(format!("  {:^12}  ", preview_val), field_style(2)),
    ]);
    frame.render_widget(Paragraph::new(preview_line), chunks[5]);

    // Field 3: Default Path
    let path_val = if editing_path {
        state.map(|s| s.path_input.as_str()).unwrap_or("")
    } else {
        &context.config.general.default_path
    };
    let path_suffix = if editing_path && focused == 3 { "▎" } else { "" };
    let path_line = Line::from(vec![
        Span::styled("  Default Path   ", Style::default().fg(Color::Yellow)),
        Span::styled(format!(" {}{} ", path_val, path_suffix), field_style(3)),
    ]);
    frame.render_widget(Paragraph::new(path_line), chunks[7]);

    // Hint
    let hint = if editing_path {
        "Enter: confirm  Esc: cancel"
    } else {
        "↑↓: navigate  ◂▸/Space: change  Enter: edit path  Esc: save & close"
    };
    let hint_line = Paragraph::new(hint)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(hint_line, chunks[9]);

    Ok(())
}

/// Renders the connection dialog popup
pub fn popup_connect_dialog(frame: &mut Frame, context: &mut Context) -> RenderResult<()> {
    let dialog = match context.connect_dialog.as_ref() {
        Some(d) => d,
        None => return Ok(()),
    };

    let r = frame.area();
    let width = (r.width * 70 / 100).clamp(40, 70);
    let height = 22_u16.min(r.height.saturating_sub(2));
    let x = (r.width.saturating_sub(width)) / 2;
    let y = (r.height.saturating_sub(height)) / 2;
    let area = Rect::new(r.x + x, r.y + y, width, height);

    let dialog_block = Block::default()
        .title(" Connect to Remote Server ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(Clear, area);
    frame.render_widget(dialog_block.clone(), area);

    let inner = dialog_block.inner(area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // 0: Protocol
            Constraint::Length(1), // 1: spacing
            Constraint::Length(3), // 2: Host
            Constraint::Length(3), // 3: Port
            Constraint::Length(3), // 4: Username
            Constraint::Length(3), // 5: Password
            Constraint::Length(3), // 6: Key path
            Constraint::Min(1),   // 7: Error / hint (flexible)
        ])
        .split(inner);

    let focused = dialog.focused_field;

    // Protocol selector
    let proto_text = match dialog.protocol {
        crate::context::ConnectionProtocol::Sftp => "< SFTP >",
        crate::context::ConnectionProtocol::Ftp => "< FTP  >",
    };
    let proto_style = if focused == 0 {
        Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    let proto = Paragraph::new(format!("  Protocol: {}", proto_text)).style(proto_style);
    frame.render_widget(proto, chunks[0]);

    // Render fields
    let fields = [
        ("Host", &dialog.host, false),
        ("Port", &dialog.port, false),
        ("Username", &dialog.username, false),
        ("Password", &dialog.password, true),
        ("SSH Key", &dialog.key_path, false),
    ];

    for (i, (label, value, is_password)) in fields.iter().enumerate() {
        let field_idx = i + 1;
        let is_focused = focused == field_idx;
        let border_color = if is_focused { Color::Cyan } else { Color::DarkGray };

        let display_value = if *is_password && !value.is_empty() {
            "*".repeat(value.len())
        } else {
            (*value).clone()
        };

        let cursor_suffix = if is_focused { "▎" } else { "" };

        let para = Paragraph::new(format!("{}{}", display_value, cursor_suffix))
            .block(
                Block::default()
                    .title(format!(" {} ", label))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
            )
            .style(if is_focused {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::Gray)
            });

        frame.render_widget(para, chunks[i + 2]);
    }

    // Error message or hint
    if let Some(ref err) = dialog.error_message {
        let err_para = Paragraph::new(format!("  {}", err))
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .wrap(Wrap { trim: false });
        frame.render_widget(err_para, chunks[7]);
    } else {
        let hint = Paragraph::new("  [Tab] Next  [↑↓] Protocol  [Enter] Connect  [Esc] Cancel")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(hint, chunks[7]);
    }

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
